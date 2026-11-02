use std::path::Path;

/// Whether this process runs from a macOS `.app` bundle rather than a bare build output
/// (`cargo run`, `target/release/actions-lights`). Bundle-only APIs — login items,
/// UNUserNotificationCenter — misbehave or register the wrong thing otherwise.
fn is_app_executable(exe: &Path) -> bool {
    let mut dirs = exe
        .ancestors()
        .skip(1)
        .map(|d| d.file_name().and_then(|n| n.to_str()));
    matches!(
        (dirs.next(), dirs.next(), dirs.next()),
        (Some(Some("MacOS")), Some(Some("Contents")), Some(Some(app))) if app.ends_with(".app")
    )
}

pub fn running_from_app() -> bool {
    std::env::current_exe()
        .map(|p| is_app_executable(&p))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_executables_inside_an_app_bundle_count() {
        assert!(is_app_executable(Path::new(
            "/Applications/Actions Lights.app/Contents/MacOS/actions-lights"
        )));
        assert!(!is_app_executable(Path::new(
            "/Users/me/actions-lights/src-tauri/target/release/actions-lights"
        )));
        assert!(!is_app_executable(Path::new(
            "/Users/me/my.app.backup/actions-lights"
        )));
    }
}
