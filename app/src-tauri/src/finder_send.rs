//! The Finder's "Send with PCTwin" (a Quick Send Stage 1 entry point, macOS): a service declared
//! in Info.plist (`NSServices`, message `sendFiles`, for any file or folder, shown in the Finder's
//! right-click menu with no setting to turn on), its name in each launch language
//! (`macos/<language>.lproj/ServicesMenu.strings`). macOS hands the chosen items to this app's
//! services provider on a pasteboard; their file addresses join the chosen list exactly as Open
//! With's do ([`crate::handed::opened`]). Nothing is sent before the person chooses Send.
//! Set up only with the real app (not on Tauri's mock runtime).

use std::path::PathBuf;
use std::sync::OnceLock;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSApplication, NSPasteboard, NSPasteboardTypeFileURL};
use objc2_foundation::NSString;

type Hand = Box<dyn Fn(Vec<PathBuf>) + Send + Sync>;

/// Where handed files go: the running app's chosen list.
static HAND: OnceLock<Hand> = OnceLock::new();

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "PCTwinFinderSend"]
    struct FinderSend;

    impl FinderSend {
        /// Info.plist's `NSMessage` `sendFiles`: the items the person chose in the Finder.
        #[unsafe(method(sendFiles:userData:error:))]
        fn send_files(
            &self,
            pasteboard: &NSPasteboard,
            _user_data: Option<&NSString>,
            _error: *mut *mut NSString,
        ) {
            let paths = paths_on(pasteboard);
            if let (false, Some(hand)) = (paths.is_empty(), HAND.get()) {
                hand(paths);
            }
        }
    }
);

/// The files and folders on the pasteboard, by their file addresses only (anything else is
/// ignored), each one that exists, at most [`crate::handed::MOST_PATHS`].
fn paths_on(pasteboard: &NSPasteboard) -> Vec<PathBuf> {
    let Some(items) = pasteboard.pasteboardItems() else {
        return Vec::new();
    };
    // SAFETY: an AppKit constant, set before any code runs and never changed.
    let file_url = unsafe { NSPasteboardTypeFileURL };
    let urls: Vec<tauri::Url> = items
        .iter()
        .filter_map(|item| tauri::Url::parse(&item.stringForType(file_url)?.to_string()).ok())
        .take(crate::handed::MOST_PATHS)
        .collect();
    crate::handed::opened(&urls)
}

/// Makes this app the provider of its Finder service; `hand` receives the chosen files. Called
/// once, on the main thread, when the app starts.
pub fn set_up(hand: impl Fn(Vec<PathBuf>) + Send + Sync + 'static) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    if HAND.set(Box::new(hand)).is_err() {
        return;
    }
    // SAFETY: `init` on a freshly allocated NSObject subclass with no fields of its own.
    let provider: Retained<FinderSend> = unsafe { msg_send![FinderSend::alloc(mtm), init] };
    let any: &AnyObject = &provider;
    // SAFETY: the provider answers the message Info.plist names, with AppKit's arguments.
    unsafe { NSApplication::sharedApplication(mtm).setServicesProvider(Some(any)) };
    // It serves for as long as the app runs.
    std::mem::forget(provider);
    objc2_app_kit::NSUpdateDynamicServices();
}
