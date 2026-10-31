use crate::model::Config;
use std::fs;
use std::io;
use std::path::Path;

const FILE: &str = "config.json";

/// Missing file → defaults. Unreadable/corrupt file is kept as `config.json.bak` and defaults
/// are used, so a bad edit never bricks the app.
pub fn load(dir: &Path) -> Config {
    let path = dir.join(FILE);
    let Ok(text) = fs::read_to_string(&path) else {
        return Config::default();
    };
    let mut cfg: Config = serde_json::from_str(&text).unwrap_or_else(|_| {
        let _ = fs::rename(&path, dir.join("config.json.bak"));
        Config::default()
    });
    // Older versions pre-filled branch/event; the run now tells us both.
    if !cfg.filters_cleared {
        for w in &mut cfg.watched {
            w.branch = None;
            w.event = None;
        }
        cfg.filters_cleared = true;
        let _ = save(dir, &cfg);
    }
    cfg
}

/// Atomic write: tmp file + rename.
pub fn save(dir: &Path, cfg: &Config) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let tmp = dir.join("config.json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(cfg)?)?;
    fs::rename(tmp, dir.join(FILE))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NotifyPrefs, WatchedWorkflow};

    #[test]
    fn missing_file_gives_default() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(load(tmp.path()), Config::default());
    }

    #[test]
    fn roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("nested");
        let cfg = Config {
            watched: vec![WatchedWorkflow {
                repo: "acme/api".into(),
                workflow_id: 42,
                workflow_name: "Deploy".into(),
                branch: Some("release".into()),
                event: Some("push".into()),
                notify: NotifyPrefs::default(),
            }],
            autostart: false,
            sound: false,
            filters_cleared: true,
            language: Some("fr".into()),
        };
        save(&dir, &cfg).unwrap();
        assert_eq!(load(&dir), cfg);
        assert!(!dir.join("config.json.tmp").exists());
    }

    #[test]
    fn corrupt_file_backs_up_and_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("config.json"), "{nope").unwrap();
        assert_eq!(load(tmp.path()), Config::default());
        assert!(tmp.path().join("config.json.bak").exists());
    }

    #[test]
    fn older_config_without_sound_defaults_on() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("config.json"),
            r#"{"watched":[],"autostart":true}"#,
        )
        .unwrap();
        assert!(load(tmp.path()).sound);
        assert!(Config::default().sound);
    }

    #[test]
    fn saved_filters_are_cleared_once() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("config.json"),
            r#"{"watched":[{"repo":"acme/api","workflow_id":42,"workflow_name":"Deploy",
                "branch":"main","event":"push"}],"autostart":true}"#,
        )
        .unwrap();
        let cfg = load(tmp.path());
        assert_eq!(
            (
                cfg.watched[0].branch.as_deref(),
                cfg.watched[0].event.as_deref()
            ),
            (None, None)
        );
        // Persisted, so a filter the user sets afterwards survives the next load.
        let mut kept = load(tmp.path());
        kept.watched[0].branch = Some("release".into());
        save(tmp.path(), &kept).unwrap();
        assert_eq!(
            load(tmp.path()).watched[0].branch.as_deref(),
            Some("release")
        );
    }
}
