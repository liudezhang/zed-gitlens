use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Deserialize)]
struct Message {
    id: Option<Value>,
    method: Option<String>,
    params: Option<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PositionParams {
    text_document: TextDocument,
}

#[derive(Deserialize)]
struct TextDocument {
    uri: String,
}

#[derive(Serialize)]
struct HintPosition {
    line: usize,
    character: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InlayHint {
    position: HintPosition,
    label: String,
    kind: u8,
    padding_left: bool,
    tooltip: String,
}

#[derive(Default)]
struct Blame {
    author: String,
    date: String,
    summary: String,
}

fn main() {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut stdout = io::BufWriter::new(io::stdout().lock());

    while let Ok(Some(body)) = read_message(&mut reader) {
        let Ok(message) = serde_json::from_slice::<Message>(&body) else {
            continue;
        };
        let Some(method) = message.method.as_deref() else {
            continue;
        };
        let result = match method {
            "initialize" => json!({
                "capabilities": {
                    "textDocumentSync": 1,
                    "inlayHintProvider": true
                },
                "serverInfo": { "name": "gitlens-blame", "version": env!("CARGO_PKG_VERSION") }
            }),
            "textDocument/inlayHint" => {
                let hints = message.params
                    .and_then(|params| serde_json::from_value::<PositionParams>(params).ok())
                    .map(|params| hints_for_uri(&params.text_document.uri))
                    .unwrap_or_default();
                json!(hints)
            }
            "shutdown" => Value::Null,
            "exit" => break,
            _ => continue,
        };

        if let Some(id) = message.id {
            let response = json!({ "jsonrpc": "2.0", "id": id, "result": result });
            if write_message(&mut stdout, &response).is_err() {
                break;
            }
        }
    }
}

fn read_message(reader: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    let Some(length) = content_length else {
        return Ok(None);
    };
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    Ok(Some(body))
}

fn write_message(writer: &mut impl Write, value: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(value)?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)?;
    writer.flush()
}

fn hints_for_uri(uri: &str) -> Vec<InlayHint> {
    let Some(path) = file_path_from_uri(uri) else {
        return Vec::new();
    };
    let root = std::env::var_os("GITLENS_WORKDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| path.parent().unwrap_or(Path::new(".")).to_path_buf());
    if !path.starts_with(&root) {
        return Vec::new();
    }
    let relative = match path.strip_prefix(&root) {
        Ok(relative) => relative,
        Err(_) => return Vec::new(),
    };
    let git = std::env::var("GITLENS_GIT").unwrap_or_else(|_| "git".into());
    let output = Command::new(git)
        .args(["-C"])
        .arg(&root)
        .args(["blame", "--line-porcelain", "--"])
        .arg(relative)
        .output();
    let Ok(output) = output else { return Vec::new() };
    if !output.status.success() {
        return Vec::new();
    }
    let Ok(text) = String::from_utf8(output.stdout) else { return Vec::new() };
    let Ok(source) = std::fs::read_to_string(&path) else { return Vec::new() };
    parse_blame(&text)
        .into_iter()
        .zip(source.lines())
        .enumerate()
        .filter(|(_, (blame, _))| !blame.author.is_empty())
        .map(|(line, (blame, source_line))| InlayHint {
            position: HintPosition {
                line,
                character: source_line.encode_utf16().count(),
            },
            label: format!("{} · {} · {}", blame.author, blame.date, blame.summary),
            kind: 2,
            padding_left: true,
            tooltip: format!("{}\n{}", blame.date, blame.summary),
        })
        .collect()
}

fn parse_blame(text: &str) -> Vec<Blame> {
    let mut entries = Vec::new();
    let mut current: Option<Blame> = None;
    for line in text.lines() {
        if line.starts_with('\t') {
            entries.push(current.take().unwrap_or_default());
            continue;
        }
        if line.split_whitespace().count() == 4
            && line.as_bytes().first().is_some_and(u8::is_ascii_hexdigit)
        {
            current = Some(Blame::default());
            continue;
        }
        let Some((key, value)) = line.split_once(' ') else { continue };
        let Some(blame) = current.as_mut() else { continue };
        match key {
            "author" => blame.author = value.to_string(),
            "author-time" => blame.date = format_timestamp(value),
            "summary" => blame.summary = value.to_string(),
            _ => {}
        }
    }
    entries
}

fn format_timestamp(value: &str) -> String {
    value.parse::<i64>()
        .ok()
        .and_then(|seconds| std::time::UNIX_EPOCH.checked_add(std::time::Duration::from_secs(seconds as u64)))
        .map(|time| {
            let elapsed = std::time::SystemTime::now()
                .duration_since(time)
                .unwrap_or_default()
                .as_secs();
            if elapsed < 86_400 {
                format!("{}h ago", elapsed / 3_600)
            } else {
                format!("{}d ago", elapsed / 86_400)
            }
        })
        .unwrap_or_default()
}

fn file_path_from_uri(uri: &str) -> Option<PathBuf> {
    let path = uri.strip_prefix("file://")?;
    #[cfg(windows)]
    let path = path.strip_prefix('/').unwrap_or(path);
    Some(PathBuf::from(path.replace("%20", " ")))
}
