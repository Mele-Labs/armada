//! What the OS shows for a Job that needs him: a banner with a sound, and the Dock count.
//!
//! gpui posts a system notification and routes a press on one back (`App::on_system_notification_response`),
//! but its notification carries no sound, so the post is made here through `UNUserNotificationCenter`
//! and gpui's delegate still hears the press. **The sound is the notification's own**, a file in the
//! bundle's Resources, so Focus and the per-app sound setting mute it with the banner.
//!
//! macOS shows a notification only for a running `.app` with a bundle identifier. From a bare binary
//! (`cargo run`) nothing is posted and nothing aborts.

use std::sync::atomic::{AtomicBool, Ordering};

use block2::RcBlock;
use objc2::runtime::Bool;
use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;
use objc2_foundation::{NSBundle, NSError, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNMutableNotificationContent, UNNotificationRequest, UNNotificationSound, UNUserNotificationCenter,
};

use crate::log::Note;

/// Permission is macOS's and is asked once, by the first post; the answer is the person's.
static ASKED: AtomicBool = AtomicBool::new(false);

/// Whether this process is a bundled app, which is when a notification can be posted.
pub fn can_post() -> bool {
    NSBundle::mainBundle().bundleIdentifier().is_some()
}

/// Post one banner. `tag` is its identity: a newer banner with the same tag replaces the older, and a
/// press carries the tag back. `sound` is the file's name in the bundle's Resources, extension included.
pub fn post(tag: &str, title: &str, body: &str, sound: &str) {
    if !can_post() {
        Note::info("no banner: not running from an app bundle").field("tag", tag).emit();
        return;
    }
    let center = UNUserNotificationCenter::currentNotificationCenter();
    if !ASKED.swap(true, Ordering::Relaxed) {
        let answered = RcBlock::new(|granted: Bool, _why: *mut NSError| {
            if !granted.as_bool() {
                Note::info("banners are not allowed for this app").emit();
            }
        });
        center.requestAuthorizationWithOptions_completionHandler(UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound, &answered);
    }
    let content = UNMutableNotificationContent::new();
    content.setTitle(&NSString::from_str(title));
    content.setBody(&NSString::from_str(body));
    content.setSound(Some(&UNNotificationSound::soundNamed(&NSString::from_str(sound))));
    // A nil trigger delivers at once.
    let request = UNNotificationRequest::requestWithIdentifier_content_trigger(&NSString::from_str(tag), &content, None);
    let delivered = RcBlock::new(|why: *mut NSError| {
        if !why.is_null() {
            Note::warn("a banner was not delivered").emit();
        }
    });
    center.addNotificationRequest_withCompletionHandler(&request, Some(&delivered));
}

/// The Dock tile's badge: the count, or none at 0.
pub fn badge_label(count: usize) -> Option<String> {
    (count > 0).then(|| count.to_string())
}

/// Set the Dock badge. Only the main thread may; anywhere else it does nothing.
pub fn set_badge(count: usize) {
    let Some(main) = MainThreadMarker::new() else { return };
    let label = badge_label(count).map(|text| NSString::from_str(&text));
    NSApplication::sharedApplication(main).dockTile().setBadgeLabel(label.as_deref());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_badge_is_empty_at_zero_and_the_count_after() {
        assert_eq!(badge_label(0), None);
        assert_eq!(badge_label(1), Some("1".into()));
        assert_eq!(badge_label(12), Some("12".into()));
    }
}
