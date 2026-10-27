use crate::model::TokenSource;
use std::path::PathBuf;
use std::process::Command;

const SERVICE: &str = "actions-lights";
const ACCOUNT: &str = "github-token";

pub trait SecretStore {
    fn get(&self) -> Option<String>;
    fn set(&self, token: &str) -> Result<(), String>;
    fn clear(&self) -> Result<(), String>;
}

/// OS keychain (Keychain / Credential Manager / Secret Service).
pub struct KeyringStore;

impl KeyringStore {
    fn entry() -> Result<keyring::Entry, String> {
        keyring::Entry::new(SERVICE, ACCOUNT).map_err(|e| e.to_string())
    }
}

impl SecretStore for KeyringStore {
    fn get(&self) -> Option<String> {
        Self::entry().ok()?.get_password().ok().filter(|t| !t.is_empty())
    }
    fn set(&self, token: &str) -> Result<(), String> {
        Self::entry()?.set_password(token).map_err(|e| e.to_string())
    }
    fn clear(&self) -> Result<(), String> {
        match Self::entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

const GH_BIN: &str = if cfg!(windows) { "gh.exe" } else { "gh" };

/// GUI apps on macOS don't inherit the shell PATH, so check Homebrew locations first.
pub fn gh_candidates(path_env: Option<&str>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let fixed = ["/opt/homebrew/bin", "/usr/local/bin"].map(PathBuf::from);
    let from_env = path_env.map(|p| std::env::split_paths(p).collect::<Vec<_>>()).unwrap_or_default();
    for dir in fixed.into_iter().chain(from_env) {
        let p = dir.join(GH_BIN);
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

pub fn gh_token(candidates: &[PathBuf]) -> Option<String> {
    let bin = candidates.iter().find(|p| p.is_file())?;
    let out = Command::new(bin).args(["auth", "token", "--hostname", "github.com"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let token = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!token.is_empty()).then_some(token)
}

/// A manually pasted token is an explicit override, so it wins over the gh session.
pub fn resolve(store: &dyn SecretStore, gh: &dyn Fn() -> Option<String>) -> (Option<String>, TokenSource) {
    if let Some(t) = store.get() {
        return (Some(t), TokenSource::Manual);
    }
    match gh() {
        Some(t) => (Some(t), TokenSource::Gh),
        None => (None, TokenSource::None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct MemStore(RefCell<Option<String>>);

    impl SecretStore for MemStore {
        fn get(&self) -> Option<String> {
            self.0.borrow().clone()
        }
        fn set(&self, token: &str) -> Result<(), String> {
            *self.0.borrow_mut() = Some(token.into());
            Ok(())
        }
        fn clear(&self) -> Result<(), String> {
            *self.0.borrow_mut() = None;
            Ok(())
        }
    }

    fn store(v: Option<&str>) -> MemStore {
        MemStore(RefCell::new(v.map(Into::into)))
    }

    #[test]
    fn manual_wins_over_gh() {
        let r = resolve(&store(Some("m")), &|| Some("g".into()));
        assert_eq!(r, (Some("m".into()), TokenSource::Manual));
    }

    #[test]
    fn gh_when_no_manual() {
        let r = resolve(&store(None), &|| Some("g".into()));
        assert_eq!(r, (Some("g".into()), TokenSource::Gh));
    }

    #[test]
    fn none_when_nothing() {
        assert_eq!(resolve(&store(None), &|| None), (None, TokenSource::None));
    }

    #[test]
    fn candidates_order_and_dedupe() {
        let c = gh_candidates(Some("/usr/local/bin:/x"));
        assert_eq!(
            c,
            vec![
                PathBuf::from("/opt/homebrew/bin/gh"),
                PathBuf::from("/usr/local/bin/gh"),
                PathBuf::from("/x/gh"),
            ]
        );
    }

    #[cfg(unix)]
    fn script(dir: &std::path::Path, name: &str, body: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p
    }

    #[cfg(unix)]
    #[test]
    fn gh_token_uses_first_existing_candidate() {
        let tmp = tempfile::tempdir().unwrap();
        let ok = script(tmp.path(), "gh", r#"[ "$1 $2 $3 $4" = "auth token --hostname github.com" ] && echo tok"#);
        let missing = tmp.path().join("nope/gh");
        assert_eq!(gh_token(&[missing, ok]).as_deref(), Some("tok"));
    }

    #[cfg(unix)]
    #[test]
    fn gh_token_failure_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        let bad = script(tmp.path(), "gh", "exit 1");
        assert_eq!(gh_token(&[bad]), None);
        let empty = script(tmp.path(), "gh2", "echo '  '");
        assert_eq!(gh_token(&[empty]), None);
    }
}
