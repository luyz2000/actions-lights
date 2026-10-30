use crate::app_state::AppCtx;
use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

const FIRST_CHECK: Duration = Duration::from_secs(10);
const EVERY: Duration = Duration::from_secs(6 * 60 * 60);

/// A newer release published on GitHub, as shown in the popover.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
}

impl UpdateInfo {
    pub fn new(version: &str, body: Option<&str>) -> Self {
        Self {
            version: version.trim_start_matches('v').to_string(),
            notes: body
                .map(str::trim)
                .filter(|b| !b.is_empty())
                .map(String::from),
        }
    }
}

/// Asks GitHub for a newer release and publishes the result to the popover.
pub async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    let info = update.map(|u| UpdateInfo::new(&u.version, u.body.as_deref()));
    *app.state::<AppCtx>().update.write().unwrap() = info.clone();
    let _ = app.emit("update", &info);
    Ok(info)
}

/// Downloads and installs the pending release, then restarts into it.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("No hay ninguna actualización pendiente")?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    app.restart();
}

/// Background checks: shortly after launch, then every few hours.
pub fn spawn(app: AppHandle) {
    // Dev builds have no published counterpart to compare against.
    if cfg!(debug_assertions) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            if let Err(e) = check(&app).await {
                eprintln!("[updates] check failed: {e}");
            }
            tokio::time::sleep(EVERY).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_drops_the_tag_prefix() {
        assert_eq!(UpdateInfo::new("v1.3.0", None).version, "1.3.0");
        assert_eq!(UpdateInfo::new("1.3.0", None).version, "1.3.0");
    }

    #[test]
    fn blank_notes_are_none() {
        assert_eq!(UpdateInfo::new("1.3.0", Some("  \n")).notes, None);
        assert_eq!(
            UpdateInfo::new("1.3.0", Some(" Novedades\n"))
                .notes
                .as_deref(),
            Some("Novedades")
        );
    }
}
