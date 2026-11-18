use zed_extension_api::{self as zed, Command, Extension, LanguageServerId, Result, Worktree};

struct GitLensExtension {
    cached_binary: Option<String>,
}

impl Extension for GitLensExtension {
    fn new() -> Self {
        Self { cached_binary: None }
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        let binary = match &self.cached_binary {
            Some(path) => path.clone(),
            None => {
                let binary = "C:\\Users\\Administrator\\AppData\\Local\\Zed\\extensions\\work\\gitlens\\gitlens-blame.exe".to_string();
                self.cached_binary = Some(binary.clone());
                binary
            }
        };
        let git = worktree.which("git").unwrap_or_else(|| "git".to_string());

        Ok(Command {
            command: binary,
            args: Vec::new(),
            env: vec![
                ("GITLENS_WORKDIR".to_string(), worktree.root_path().to_string()),
                ("GITLENS_GIT".to_string(), git),
            ],
        })
    }
}


zed::register_extension!(GitLensExtension);
