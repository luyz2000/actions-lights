use std::path::{Path, PathBuf};

/// "owner/repo" for any common GitHub remote URL form (https, ssh, scp-like).
pub fn parse_github_remote(url: &str) -> Option<String> {
    let url = url.trim();
    let rest = if let Some(i) = url.find("github.com/") {
        &url[i + "github.com/".len()..]
    } else if let Some(i) = url.find("github.com:") {
        &url[i + "github.com:".len()..]
    } else {
        return None;
    };
    let rest = rest.trim_end_matches('/');
    let rest = rest.strip_suffix(".git").unwrap_or(rest);
    let mut parts = rest.split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(owner), Some(repo), None) if !owner.is_empty() && !repo.is_empty() => Some(format!("{owner}/{repo}")),
        _ => None,
    }
}

/// URL of `remote "origin"` in a git config file, else the first remote's URL.
pub fn origin_url(config: &str) -> Option<String> {
    let mut current: Option<String> = None;
    let mut first: Option<String> = None;
    for line in config.lines().map(str::trim) {
        if line.starts_with('[') {
            current = line.strip_prefix("[remote \"").and_then(|l| l.strip_suffix("\"]")).map(str::to_string);
        } else if let (Some(remote), Some(value)) = (&current, line.strip_prefix("url").map(str::trim_start)) {
            let Some(url) = value.strip_prefix('=').map(|v| v.trim().to_string()) else { continue };
            if remote == "origin" {
                return Some(url);
            }
            first.get_or_insert(url);
        }
    }
    first
}

fn git_dir(folder: &Path) -> Option<PathBuf> {
    let dot_git = folder.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git);
    }
    let text = std::fs::read_to_string(&dot_git).ok()?;
    let dir = PathBuf::from(text.trim().strip_prefix("gitdir:")?.trim());
    let dir = if dir.is_relative() { folder.join(dir) } else { dir };
    // A linked worktree keeps its config in the common dir.
    match std::fs::read_to_string(dir.join("commondir")) {
        Ok(common) => Some(dir.join(common.trim())),
        Err(_) => Some(dir),
    }
}

/// Picks the GitHub repo of a local checkout, without needing the git binary.
pub fn repo_from_folder(folder: &Path) -> Result<String, String> {
    let dir = git_dir(folder).ok_or("La carpeta no es un repositorio git")?;
    let config = std::fs::read_to_string(dir.join("config")).map_err(|_| "No se pudo leer la configuración de git")?;
    let url = origin_url(&config).ok_or("El repositorio no tiene remotos configurados")?;
    parse_github_remote(&url).ok_or_else(|| format!("El remoto no es de GitHub: {url}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_github_remote_forms() {
        for url in [
            "https://github.com/acme/api.git",
            "https://github.com/acme/api",
            "https://user@github.com/acme/api.git",
            "git@github.com:acme/api.git",
            "ssh://git@github.com/acme/api",
            "https://github.com/acme/api/",
        ] {
            assert_eq!(parse_github_remote(url).as_deref(), Some("acme/api"), "{url}");
        }
    }

    #[test]
    fn rejects_non_github_remotes() {
        assert_eq!(parse_github_remote("https://gitlab.com/acme/api.git"), None);
        assert_eq!(parse_github_remote("git@github.com:acme"), None);
        assert_eq!(parse_github_remote(""), None);
    }

    #[test]
    fn origin_url_from_config_prefers_origin() {
        let cfg = r#"
[core]
	bare = false
[remote "upstream"]
	url = git@github.com:other/api.git
[remote "origin"]
	url = https://github.com/acme/api.git
	fetch = +refs/heads/*:refs/remotes/origin/*
"#;
        assert_eq!(origin_url(cfg).as_deref(), Some("https://github.com/acme/api.git"));
    }

    #[test]
    fn origin_url_falls_back_to_first_remote() {
        let cfg = "[remote \"upstream\"]\n\turl = git@github.com:other/api.git\n";
        assert_eq!(origin_url(cfg).as_deref(), Some("git@github.com:other/api.git"));
    }

    #[test]
    fn repo_from_folder_reads_git_config() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        std::fs::write(tmp.path().join(".git/config"), "[remote \"origin\"]\n\turl = git@github.com:acme/api.git\n").unwrap();
        assert_eq!(repo_from_folder(tmp.path()), Ok("acme/api".to_string()));
    }

    #[test]
    fn repo_from_folder_follows_gitdir_file() {
        // Worktrees/submodules: `.git` is a file pointing at the real git dir.
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("real.git");
        std::fs::create_dir(&real).unwrap();
        std::fs::write(real.join("config"), "[remote \"origin\"]\n\turl = https://github.com/acme/web\n").unwrap();
        let work = tmp.path().join("work");
        std::fs::create_dir(&work).unwrap();
        std::fs::write(work.join(".git"), format!("gitdir: {}\n", real.display())).unwrap();
        assert_eq!(repo_from_folder(&work), Ok("acme/web".to_string()));
    }

    #[test]
    fn repo_from_folder_errors_are_spanish() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(repo_from_folder(tmp.path()), Err("La carpeta no es un repositorio git".to_string()));
    }
}
