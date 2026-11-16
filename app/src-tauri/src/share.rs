//! The macOS share menu (`NSSharingServicePicker`) for a file: Messages, Mail, AirDrop, Photos
//! and whatever other apps offer themselves there.

use std::path::PathBuf;

use crate::error::AppResult;

#[cfg(target_os = "macos")]
mod macos {
    use std::{cell::RefCell, path::PathBuf};

    use objc2::{AnyThread, rc::Retained, runtime::AnyObject};
    use objc2_app_kit::{NSSharingServicePicker, NSWindow};
    use objc2_foundation::{NSArray, NSPoint, NSRect, NSRectEdge, NSSize, NSString, NSURL};

    use crate::error::{AppError, AppResult};

    thread_local! {
        /// The open menu, kept alive until the next one replaces it.
        static ACTIVE: RefCell<Option<Retained<NSSharingServicePicker>>> = const { RefCell::new(None) };
    }

    pub fn present(
        window: &tauri::WebviewWindow,
        file: PathBuf,
        anchor: [f64; 4],
    ) -> AppResult<()> {
        let parent = window
            .ns_window()
            .map_err(|e| AppError::Invalid(format!("no window: {e}")))?
            as usize;
        window
            .run_on_main_thread(move || {
                let Some(parent) = (unsafe { Retained::retain(parent as *mut NSWindow) }) else {
                    return;
                };
                let Some(view) = parent.contentView() else {
                    return;
                };
                let url = NSURL::fileURLWithPath(&NSString::from_str(&file.to_string_lossy()));
                let items = NSArray::<AnyObject>::from_retained_slice(&[url.into_super().into()]);
                let picker = unsafe {
                    NSSharingServicePicker::initWithItems(NSSharingServicePicker::alloc(), &items)
                };
                // `anchor` is the button in web coordinates (top-left origin); AppKit views count
                // from the bottom unless they are flipped.
                let [x, y, width, height] = anchor;
                let y = if view.isFlipped() {
                    y
                } else {
                    view.bounds().size.height - y - height
                };
                let rect = NSRect::new(NSPoint::new(x, y), NSSize::new(width, height));
                picker.showRelativeToRect_ofView_preferredEdge(rect, &view, NSRectEdge::MinY);
                ACTIVE.with(|active| *active.borrow_mut() = Some(picker));
            })
            .map_err(|e| AppError::Invalid(format!("could not show the share menu: {e}")))
    }
}

/// Shows the share menu for `file` next to `anchor` (x, y, width, height in the window's web
/// coordinates).
pub fn present(window: &tauri::WebviewWindow, file: PathBuf, anchor: [f64; 4]) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    return macos::present(window, file, anchor);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, file, anchor);
        Err(crate::error::AppError::Invalid(
            "the share menu is only available on macOS".into(),
        ))
    }
}
