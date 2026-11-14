//! "Add from Photos…": Apple's photo picker (PhotoKit's `PHPickerViewController`, macOS 13+)
//! shown as a sheet on the app window. The picker only hands over what the user picks, so
//! RunningMate needs no access to the whole Photos library. Picked items arrive as temporary
//! files, which are copied to a staging folder before PhotoKit deletes them.

use std::path::PathBuf;

/// Called once with the staged copies (empty when the user cancelled).
pub type Done = Box<dyn FnOnce(Vec<PathBuf>) + Send>;

#[cfg(target_os = "macos")]
mod macos {
    use std::{
        cell::RefCell,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };

    use block2::RcBlock;
    use objc2::{
        DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
        runtime::ProtocolObject,
    };
    use objc2_app_kit::NSWindow;
    use objc2_foundation::{NSArray, NSError, NSObject, NSObjectProtocol, NSSize, NSString, NSURL};
    use objc2_photos_ui::{
        PHPickerConfiguration, PHPickerConfigurationAssetRepresentationMode, PHPickerFilter,
        PHPickerResult, PHPickerViewController, PHPickerViewControllerDelegate,
    };

    use super::Done;
    use crate::error::{AppError, AppResult};

    struct Ivars {
        parent: Retained<NSWindow>,
        sheet: RefCell<Option<Retained<NSWindow>>>,
        staging: PathBuf,
        done: RefCell<Option<Done>>,
    }

    define_class!(
        // SAFETY: NSObject has no subclassing requirements and the class doesn't implement Drop.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[ivars = Ivars]
        struct PickerDelegate;

        unsafe impl NSObjectProtocol for PickerDelegate {}

        unsafe impl PHPickerViewControllerDelegate for PickerDelegate {
            #[unsafe(method(picker:didFinishPicking:))]
            fn picker_did_finish_picking(
                &self,
                _picker: &PHPickerViewController,
                results: &NSArray<PHPickerResult>,
            ) {
                if let Some(sheet) = self.ivars().sheet.borrow_mut().take() {
                    self.ivars().parent.endSheet(&sheet);
                }
                if let Some(done) = self.ivars().done.borrow_mut().take() {
                    load_all(results.to_vec(), self.ivars().staging.clone(), done);
                }
            }
        }
    );

    impl PickerDelegate {
        fn new(
            mtm: MainThreadMarker,
            parent: Retained<NSWindow>,
            staging: PathBuf,
            done: Done,
        ) -> Retained<Self> {
            let this = Self::alloc(mtm).set_ivars(Ivars {
                parent,
                sheet: RefCell::new(None),
                staging,
                done: RefCell::new(Some(done)),
            });
            unsafe { msg_send![super(this), init] }
        }
    }

    thread_local! {
        /// The picker only holds its delegate weakly; this keeps the current one alive. It's
        /// replaced by the next picker rather than released from inside its own callback.
        static ACTIVE: RefCell<Option<Retained<PickerDelegate>>> = const { RefCell::new(None) };
    }

    /// `dir/name`, or `dir/name-N.ext` if taken.
    fn free_path(dir: &Path, stem: &str, extension: &str) -> PathBuf {
        let candidate = dir.join(format!("{stem}.{extension}"));
        if !candidate.exists() {
            return candidate;
        }
        (1..)
            .map(|n| dir.join(format!("{stem}-{n}.{extension}")))
            .find(|p| !p.exists())
            .expect("an unused file name exists")
    }

    /// Copies every picked item out of PhotoKit's temporary files, then calls `done` once.
    fn load_all(results: Vec<Retained<PHPickerResult>>, staging: PathBuf, done: Done) {
        if results.is_empty() {
            done(Vec::new());
            return;
        }
        let state = Arc::new(Mutex::new((results.len(), Vec::new(), Some(done))));
        for (index, result) in results.iter().enumerate() {
            let provider = unsafe { result.itemProvider() };
            let movie = NSString::from_str("public.movie");
            let kind = if provider.hasItemConformingToTypeIdentifier(&movie) {
                movie
            } else {
                NSString::from_str("public.image")
            };
            let stem = provider
                .suggestedName()
                .map(|n| n.to_string())
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| format!("photo-{}", index + 1));
            let state = state.clone();
            let staging = staging.clone();
            // PhotoKit deletes the file at `url` when this returns, so copy it right here.
            let handler = RcBlock::new(move |url: *mut NSURL, _error: *mut NSError| {
                let copied = unsafe { url.as_ref() }
                    .and_then(|url| url.path())
                    .and_then(|path| {
                        let source = PathBuf::from(path.to_string());
                        let extension = source.extension()?.to_string_lossy().into_owned();
                        std::fs::create_dir_all(&staging).ok()?;
                        let target = free_path(&staging, &stem, &extension);
                        std::fs::copy(&source, &target).ok()?;
                        Some(target)
                    });
                let mut state = state.lock().expect("picker state");
                state.0 -= 1;
                state.1.extend(copied);
                if state.0 == 0
                    && let Some(done) = state.2.take()
                {
                    done(std::mem::take(&mut state.1));
                }
            });
            unsafe {
                provider.loadFileRepresentationForTypeIdentifier_completionHandler(&kind, &handler)
            };
        }
    }

    pub fn present(window: &tauri::WebviewWindow, staging: PathBuf, done: Done) -> AppResult<()> {
        let parent = window
            .ns_window()
            .map_err(|e| AppError::Invalid(format!("no window: {e}")))?
            as usize;
        window
            .run_on_main_thread(move || {
                let mtm = MainThreadMarker::new().expect("runs on the main thread");
                let Some(parent) = (unsafe { Retained::retain(parent as *mut NSWindow) }) else {
                    done(Vec::new());
                    return;
                };
                let config = unsafe { PHPickerConfiguration::new() };
                unsafe {
                    // 0 = no limit; "compatible" hands over JPEG/H.264 instead of HEIC/HEVC.
                    config.setSelectionLimit(0);
                    config.setPreferredAssetRepresentationMode(
                        PHPickerConfigurationAssetRepresentationMode::Compatible,
                    );
                    let filters = NSArray::from_retained_slice(&[
                        PHPickerFilter::imagesFilter(),
                        PHPickerFilter::videosFilter(),
                    ]);
                    config.setFilter(Some(&PHPickerFilter::anyFilterMatchingSubfilters(&filters)));
                }
                let picker = unsafe {
                    PHPickerViewController::initWithConfiguration(
                        PHPickerViewController::alloc(mtm),
                        &config,
                    )
                };
                let delegate = PickerDelegate::new(mtm, parent.clone(), staging, done);
                unsafe { picker.setDelegate(Some(ProtocolObject::from_ref(&*delegate))) };
                let sheet = NSWindow::windowWithContentViewController(&picker);
                sheet.setContentSize(NSSize::new(860.0, 600.0));
                *delegate.ivars().sheet.borrow_mut() = Some(sheet.clone());
                parent.beginSheet_completionHandler(&sheet, None);
                ACTIVE.with(|active| *active.borrow_mut() = Some(delegate));
            })
            .map_err(|e| AppError::Invalid(format!("could not show the photo picker: {e}")))
    }
}

/// Shows the picker over `window`; `done` receives copies of the picked items in `staging`.
pub fn present(
    window: &tauri::WebviewWindow,
    staging: PathBuf,
    done: Done,
) -> crate::error::AppResult<()> {
    #[cfg(target_os = "macos")]
    return macos::present(window, staging, done);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, staging, done);
        Err(crate::error::AppError::Invalid(
            "the Photos library is only available on macOS".into(),
        ))
    }
}
