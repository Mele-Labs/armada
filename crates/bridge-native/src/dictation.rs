//! macOS Dictation, the third Talk voice: the system's own, started in whichever box has focus.
//! It types into that box itself, so the words never pass through here. This is the one file in
//! the crate that speaks `unsafe` (`xtask/src/rules_unsafe.rs` lists it): objc2-app-kit declares
//! the call it needs unsafe, and has no safe method that sends an action by selector.

use objc2::{sel, MainThreadMarker};
use objc2_app_kit::NSApplication;

/// Sends `startDictation:` or `stopDictation:` to the first responder. False where nothing in
/// the responder chain answers it, or off the main thread.
#[allow(unsafe_code)]
pub fn dictate(start: bool) -> bool {
    let Some(main) = MainThreadMarker::new() else { return false };
    let app = NSApplication::sharedApplication(main);
    let action = if start { sel!(startDictation:) } else { sel!(stopDictation:) };
    // SAFETY: `action` is a selector the Objective-C runtime has interned, a target of `None` is
    // the documented way to address the first responder, and the sender is not read by either
    // action. The call happens on the main thread, which `main` proves.
    unsafe { app.sendAction_to_from(action, None, None) }
}
