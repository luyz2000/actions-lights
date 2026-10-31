use std::sync::Mutex;
use tauri::AppHandle;

/// A notification about a run; clicking it opens `url` where the platform reports clicks.
pub struct Outgoing {
    pub title: String,
    pub body: String,
    pub sound: Option<&'static str>,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub url: Option<String>,
}

/// Off until the app is signed with an Apple Developer ID: on macOS 26 an ad-hoc signed app
/// gets no notifications at all (see docs/notificaciones-macos.md, kept out of git).
pub const NOTIFICATIONS_ENABLED: bool = false;

/// Run URLs waiting for a click, by notification id. Only macOS reports clicks.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
static PENDING: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn remember(id: &str, url: &str) {
    PENDING
        .lock()
        .unwrap()
        .push((id.to_string(), url.to_string()));
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn take(id: &str) -> Option<String> {
    let mut pending = PENDING.lock().unwrap();
    let i = pending.iter().position(|(pid, _)| pid == id)?;
    Some(pending.swap_remove(i).1)
}

/// Whether system notifications can be shown, for the settings window.
#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub struct NotificationStatus {
    pub available: bool,
    pub reason: Option<String>,
}

impl NotificationStatus {
    /// `answer`: the system's reply to the permission request, if it has arrived.
    pub fn from_answer(answer: Option<Result<(), String>>) -> Self {
        match answer {
            Some(Err(reason)) => Self {
                available: false,
                reason: Some(reason),
            },
            _ => Self {
                available: true,
                reason: None,
            },
        }
    }
}

/// The system's reply to the permission request (macOS native notifications only).
static ANSWER: Mutex<Option<Result<(), String>>> = Mutex::new(None);

pub fn status() -> NotificationStatus {
    if !NOTIFICATIONS_ENABLED {
        return NotificationStatus {
            available: false,
            reason: None,
        };
    }
    NotificationStatus::from_answer(ANSWER.lock().unwrap().clone())
}

/// Sets up native notifications (permission prompt, click handling) where available.
pub fn init(app: &AppHandle) {
    if !NOTIFICATIONS_ENABLED {
        return;
    }
    #[cfg(target_os = "macos")]
    mac::init(app);
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

pub fn show(app: &AppHandle, n: Outgoing) {
    if !NOTIFICATIONS_ENABLED {
        return;
    }
    #[cfg(target_os = "macos")]
    if mac::ready() {
        mac::show(n);
        return;
    }
    use tauri_plugin_notification::NotificationExt;
    let mut builder = app.notification().builder().title(n.title).body(n.body);
    if let Some(sound) = n.sound {
        builder = builder.sound(sound);
    }
    let _ = builder.show();
}

/// macOS: UNUserNotificationCenter registers the app in System Settings › Notifications and
/// reports clicks; the deprecated NSUserNotification path used by the plugin does neither.
#[cfg(target_os = "macos")]
mod mac {
    use super::{ANSWER, Outgoing, remember, take};
    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::{Bool, NSObject, ProtocolObject};
    use objc2::{ClassType, define_class, msg_send};
    use objc2_foundation::{NSError, NSObjectProtocol, NSString};
    use objc2_user_notifications::{
        UNAuthorizationOptions, UNMutableNotificationContent, UNNotification,
        UNNotificationPresentationOptions, UNNotificationRequest, UNNotificationResponse,
        UNUserNotificationCenter, UNUserNotificationCenterDelegate,
    };
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tauri::AppHandle;
    use tauri_plugin_opener::OpenerExt;

    static APP: OnceLock<AppHandle> = OnceLock::new();
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    define_class!(
        #[unsafe(super(NSObject))]
        #[name = "ActionsLightsNotificationDelegate"]
        struct Delegate;

        unsafe impl NSObjectProtocol for Delegate {}

        unsafe impl UNUserNotificationCenterDelegate for Delegate {
            // Show banners even while the (menu-bar) app counts as active.
            #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
            fn will_present(
                &self,
                _center: &UNUserNotificationCenter,
                _notification: &UNNotification,
                handler: &block2::DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
            ) {
                handler.call((UNNotificationPresentationOptions::Banner
                    | UNNotificationPresentationOptions::List,));
            }

            #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
            fn did_receive(
                &self,
                _center: &UNUserNotificationCenter,
                response: &UNNotificationResponse,
                handler: &block2::DynBlock<dyn Fn()>,
            ) {
                let id = response.notification().request().identifier().to_string();
                if let (Some(url), Some(app)) = (take(&id), APP.get()) {
                    let _ = app.opener().open_url(url, None::<&str>);
                }
                handler.call(());
            }
        }
    );

    /// UNUserNotificationCenter aborts outside an app bundle (e.g. `cargo run`).
    fn in_bundle() -> bool {
        std::env::current_exe()
            .map(|p| p.to_string_lossy().contains(".app/Contents/MacOS/"))
            .unwrap_or(false)
    }

    /// Native path only once the system allowed it; otherwise the plugin is used.
    pub fn ready() -> bool {
        APP.get().is_some() && matches!(*ANSWER.lock().unwrap(), Some(Ok(())))
    }

    pub fn init(app: &AppHandle) {
        if !in_bundle() || APP.set(app.clone()).is_err() {
            return;
        }
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let delegate: Retained<Delegate> = unsafe { msg_send![Delegate::class(), new] };
        center.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        // The center keeps only a weak reference; the delegate lives as long as the app.
        std::mem::forget(delegate);
        let done = RcBlock::new(|granted: Bool, error: *mut NSError| {
            let answer = if granted.as_bool() {
                Ok(())
            } else {
                // SAFETY: the system passes either null or a valid NSError.
                Err(unsafe { error.as_ref() }
                    .map(|e| e.localizedDescription().to_string())
                    .unwrap_or_else(|| "denied".into()))
            };
            if let Err(reason) = &answer {
                eprintln!("[notify] notifications not authorized: {reason}");
            }
            *ANSWER.lock().unwrap() = Some(answer);
        });
        center.requestAuthorizationWithOptions_completionHandler(
            UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
            &done,
        );
    }

    pub fn show(n: Outgoing) {
        let id = format!("run-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed));
        if let Some(url) = &n.url {
            remember(&id, url);
        }
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(&n.title));
        content.setBody(&NSString::from_str(&n.body));
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str(&id),
            &content,
            None,
        );
        UNUserNotificationCenter::currentNotificationCenter()
            .addNotificationRequest_withCompletionHandler(&request, None);
        // Named system sounds (Glass, Basso) are played directly: UNNotificationSound only
        // looks inside the app bundle and ~/Library/Sounds.
        if let Some(name) = n.sound
            && let Some(sound) = objc2_app_kit::NSSound::soundNamed(&NSString::from_str(name))
        {
            sound.play();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clicked_notification_opens_its_run_once() {
        remember("n-1", "https://github.com/o/r/actions/runs/1");
        remember("n-2", "https://github.com/o/r/actions/runs/2");
        assert_eq!(
            take("n-2").as_deref(),
            Some("https://github.com/o/r/actions/runs/2")
        );
        assert_eq!(take("n-2"), None);
        assert_eq!(take("unknown"), None);
        assert_eq!(
            take("n-1").as_deref(),
            Some("https://github.com/o/r/actions/runs/1")
        );
    }

    #[test]
    fn status_follows_the_permission_answer() {
        assert_eq!(
            NotificationStatus::from_answer(Some(Ok(()))),
            NotificationStatus {
                available: true,
                reason: None
            }
        );
        assert_eq!(
            NotificationStatus::from_answer(Some(Err("Notifications are not allowed".into()))),
            NotificationStatus {
                available: false,
                reason: Some("Notifications are not allowed".into())
            }
        );
        // Still waiting for the system: don't claim they are broken.
        assert!(NotificationStatus::from_answer(None).available);
    }
}
