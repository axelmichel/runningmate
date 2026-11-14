//! "Photos from this activity": reads the macOS Photos library (PhotoKit) to list the photos and
//! videos taken while an activity was recorded. Unlike the photo picker this needs the user's
//! permission to read the library (`NSPhotoLibraryUsageDescription` in `Info.plist`).
//!
//! Exported files are named `photos-{uuid}.{ext}` after the asset's local identifier, which is
//! how assets already added to an activity are recognised.

use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;

use crate::{db::media::MediaType, error::AppResult};

/// Whether RunningMate may read the Photos library.
// Each platform only produces some of the variants (macOS never `Unsupported`, others only that).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PhotosAccess {
    /// Not asked yet.
    NotDetermined,
    /// Blocked, e.g. by parental controls.
    Restricted,
    Denied,
    Authorized,
    /// Only the photos the user picked in the permission dialog are visible.
    Limited,
    /// Not macOS.
    Unsupported,
}

/// A photo or video in the Photos library.
// Only built on macOS; elsewhere listing the library is an error.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LibraryAsset {
    /// PhotoKit's local identifier.
    pub id: String,
    /// Creation time, Unix seconds.
    #[ts(type = "number")]
    pub date: i64,
    pub media_type: MediaType,
    /// Video length in seconds.
    #[ts(optional)]
    pub duration: Option<f64>,
    /// Already added to the activity.
    pub added: bool,
}

/// Prefix of the exported file name for an asset (`photos-{uuid}`).
pub fn file_stem(id: &str) -> String {
    let uuid = id.split('/').next().unwrap_or(id);
    format!("photos-{uuid}")
}

/// Whether one of `files` was exported from the asset `id`.
pub fn is_added<'a>(id: &str, mut files: impl Iterator<Item = &'a str>) -> bool {
    let stem = file_stem(id);
    files.any(|file| {
        Path::new(file)
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|name| name.starts_with(&stem))
    })
}

#[cfg(target_os = "macos")]
mod macos {
    use std::{
        cell::RefCell,
        path::{Path, PathBuf},
        rc::Rc,
        sync::{Mutex, mpsc},
    };

    use block2::RcBlock;
    use objc2::{rc::Retained, runtime::AnyObject};
    use objc2_app_kit::{
        NSBitmapImageFileType, NSBitmapImageRep, NSImage, NSImageCompressionFactor,
    };
    use objc2_foundation::{
        NSArray, NSDate, NSDictionary, NSError, NSNumber, NSPredicate, NSSize, NSSortDescriptor,
        NSString, NSURL,
    };
    use objc2_photos::{
        PHAccessLevel, PHAsset, PHAssetMediaType, PHAssetResource, PHAssetResourceManager,
        PHAssetResourceRequestOptions, PHAssetResourceType, PHAuthorizationStatus, PHFetchOptions,
        PHImageContentMode, PHImageManager, PHImageRequestOptions,
        PHImageRequestOptionsDeliveryMode, PHImageRequestOptionsResizeMode,
        PHImageRequestOptionsVersion, PHPhotoLibrary,
    };

    use super::{LibraryAsset, PhotosAccess};
    use crate::{
        db::media::MediaType,
        error::{AppError, AppResult},
    };

    fn access_from(status: PHAuthorizationStatus) -> PhotosAccess {
        match status {
            PHAuthorizationStatus::Restricted => PhotosAccess::Restricted,
            PHAuthorizationStatus::Denied => PhotosAccess::Denied,
            PHAuthorizationStatus::Authorized => PhotosAccess::Authorized,
            PHAuthorizationStatus::Limited => PhotosAccess::Limited,
            _ => PhotosAccess::NotDetermined,
        }
    }

    pub fn access() -> PhotosAccess {
        access_from(unsafe {
            PHPhotoLibrary::authorizationStatusForAccessLevel(PHAccessLevel::ReadWrite)
        })
    }

    /// Shows the system permission dialog (only the first time) and waits for the answer.
    pub fn request_access() -> PhotosAccess {
        let (sender, receiver) = mpsc::channel();
        let sender = Mutex::new(Some(sender));
        let handler = RcBlock::new(move |status: PHAuthorizationStatus| {
            if let Some(sender) = sender.lock().ok().and_then(|mut s| s.take()) {
                let _ = sender.send(status);
            }
        });
        unsafe {
            PHPhotoLibrary::requestAuthorizationForAccessLevel_handler(
                PHAccessLevel::ReadWrite,
                &handler,
            );
        }
        receiver
            .recv()
            .map(access_from)
            .unwrap_or_else(|_| access())
    }

    pub fn assets_between(from: i64, to: i64) -> AppResult<Vec<LibraryAsset>> {
        let options = unsafe { PHFetchOptions::new() };
        let start = NSDate::dateWithTimeIntervalSince1970(from as f64);
        let end = NSDate::dateWithTimeIntervalSince1970(to as f64);
        let arguments = NSArray::<AnyObject>::from_slice(&[&*start as &AnyObject, &*end]);
        let format = NSString::from_str("creationDate >= %@ AND creationDate <= %@");
        let sort = NSSortDescriptor::sortDescriptorWithKey_ascending(
            Some(&NSString::from_str("creationDate")),
            true,
        );
        unsafe {
            let predicate =
                NSPredicate::predicateWithFormat_argumentArray(&format, Some(&arguments));
            options.setPredicate(Some(&predicate));
            options.setSortDescriptors(Some(&NSArray::from_retained_slice(&[sort])));
        }
        let result = unsafe { PHAsset::fetchAssetsWithOptions(Some(&options)) };
        let mut assets = Vec::new();
        for index in 0..unsafe { result.count() } {
            let asset = unsafe { result.objectAtIndex(index) };
            let media_type = match unsafe { asset.mediaType() } {
                PHAssetMediaType::Image => MediaType::Image,
                PHAssetMediaType::Video => MediaType::Video,
                _ => continue,
            };
            let Some(date) = (unsafe { asset.creationDate() }) else {
                continue;
            };
            assets.push(LibraryAsset {
                id: unsafe { asset.localIdentifier() }.to_string(),
                date: date.timeIntervalSince1970().round() as i64,
                media_type,
                duration: (media_type == MediaType::Video).then(|| unsafe { asset.duration() }),
                added: false,
            });
        }
        Ok(assets)
    }

    fn fetch(id: &str) -> AppResult<Retained<PHAsset>> {
        let ids = NSArray::from_retained_slice(&[NSString::from_str(id)]);
        let result = unsafe { PHAsset::fetchAssetsWithLocalIdentifiers_options(&ids, None) };
        if unsafe { result.count() } == 0 {
            return Err(AppError::NotFound(format!("Photos item {id}")));
        }
        Ok(unsafe { result.objectAtIndex(0) })
    }

    /// Renders the asset (a video's poster frame) to fit `max` × `max` pixels, as JPEG.
    fn render_jpeg(asset: &PHAsset, max: f64, quality: f64) -> AppResult<Vec<u8>> {
        let options = unsafe { PHImageRequestOptions::new() };
        unsafe {
            // Synchronous: the handler runs once, on this thread, with the final image.
            options.setSynchronous(true);
            options.setNetworkAccessAllowed(true);
            options.setVersion(PHImageRequestOptionsVersion::Current);
            options.setDeliveryMode(PHImageRequestOptionsDeliveryMode::HighQualityFormat);
            options.setResizeMode(PHImageRequestOptionsResizeMode::Exact);
        }
        let image: Rc<RefCell<Option<Retained<NSImage>>>> = Rc::default();
        let handler = {
            let image = image.clone();
            RcBlock::new(move |result: *mut NSImage, _info: *mut NSDictionary| {
                *image.borrow_mut() = unsafe { Retained::retain(result) };
            })
        };
        unsafe {
            PHImageManager::defaultManager()
                .requestImageForAsset_targetSize_contentMode_options_resultHandler(
                    asset,
                    NSSize::new(max, max),
                    PHImageContentMode::AspectFit,
                    Some(&options),
                    &handler,
                );
        }
        let image = image
            .borrow_mut()
            .take()
            .ok_or_else(|| AppError::Invalid("Photos didn't deliver the image".into()))?;
        let jpeg = image
            .TIFFRepresentation()
            .and_then(|tiff| NSBitmapImageRep::imageRepWithData(&tiff))
            .and_then(|bitmap| {
                let quality = NSNumber::new_f64(quality);
                let properties = NSDictionary::<NSString, AnyObject>::from_slices(
                    &[unsafe { NSImageCompressionFactor }],
                    &[&*quality as &AnyObject],
                );
                unsafe {
                    bitmap.representationUsingType_properties(
                        NSBitmapImageFileType::JPEG,
                        &properties,
                    )
                }
            })
            .ok_or_else(|| AppError::Invalid("couldn't convert the image to JPEG".into()))?;
        Ok(jpeg.to_vec())
    }

    pub fn thumbnail(id: &str, target: &Path) -> AppResult<()> {
        let jpeg = render_jpeg(&*fetch(id)?, 320.0, 0.8)?;
        std::fs::write(target, jpeg)?;
        Ok(())
    }

    /// Writes the asset into `dir` as `{stem}.jpg` (photos) or the original video file, and
    /// returns its path.
    pub fn export(id: &str, dir: &Path, stem: &str) -> AppResult<PathBuf> {
        let asset = fetch(id)?;
        if unsafe { asset.mediaType() } != PHAssetMediaType::Video {
            let target = dir.join(format!("{stem}.jpg"));
            // Large enough for any screen; full 48-MP originals would be wasted space here.
            std::fs::write(&target, render_jpeg(&asset, 4096.0, 0.9)?)?;
            return Ok(target);
        }
        let resources = unsafe { PHAssetResource::assetResourcesForAsset(&asset) };
        let resource = [
            PHAssetResourceType::FullSizeVideo,
            PHAssetResourceType::Video,
        ]
        .into_iter()
        .find_map(|kind| resources.iter().find(|r| unsafe { r.r#type() } == kind))
        .ok_or_else(|| AppError::Invalid(format!("Photos item {id} has no video")))?;
        let extension = Path::new(&unsafe { resource.originalFilename() }.to_string())
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_else(|| "mov".into());
        let target = dir.join(format!("{stem}.{extension}"));
        let url = NSURL::fileURLWithPath(&NSString::from_str(&target.to_string_lossy()));
        let options = unsafe { PHAssetResourceRequestOptions::new() };
        unsafe { options.setNetworkAccessAllowed(true) };
        let (sender, receiver) = mpsc::channel();
        let sender = Mutex::new(Some(sender));
        let handler = RcBlock::new(move |error: *mut NSError| {
            let message = unsafe { error.as_ref() }.map(|e| e.localizedDescription().to_string());
            if let Some(sender) = sender.lock().ok().and_then(|mut s| s.take()) {
                let _ = sender.send(message);
            }
        });
        unsafe {
            PHAssetResourceManager::defaultManager()
                .writeDataForAssetResource_toFile_options_completionHandler(
                    &resource,
                    &url,
                    Some(&options),
                    &handler,
                );
        }
        match receiver.recv() {
            Ok(None) => Ok(target),
            Ok(Some(message)) => Err(AppError::Invalid(message)),
            Err(_) => Err(AppError::Invalid("Photos didn't export the video".into())),
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod other {
    use std::path::{Path, PathBuf};

    use super::{LibraryAsset, PhotosAccess};
    use crate::error::{AppError, AppResult};

    fn unsupported() -> AppError {
        AppError::Invalid("the Photos library is only available on macOS".into())
    }

    pub fn access() -> PhotosAccess {
        PhotosAccess::Unsupported
    }

    pub fn request_access() -> PhotosAccess {
        PhotosAccess::Unsupported
    }

    pub fn assets_between(_from: i64, _to: i64) -> AppResult<Vec<LibraryAsset>> {
        Err(unsupported())
    }

    pub fn thumbnail(_id: &str, _target: &Path) -> AppResult<()> {
        Err(unsupported())
    }

    pub fn export(_id: &str, _dir: &Path, _stem: &str) -> AppResult<PathBuf> {
        Err(unsupported())
    }
}

#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(not(target_os = "macos"))]
use other as platform;

pub fn access() -> PhotosAccess {
    platform::access()
}

/// Asks for access if it hasn't been decided yet; blocks until the user answers.
pub fn request_access() -> PhotosAccess {
    platform::request_access()
}

/// Photos and videos created between the two Unix times, oldest first.
pub fn assets_between(from: i64, to: i64) -> AppResult<Vec<LibraryAsset>> {
    platform::assets_between(from, to)
}

/// A small JPEG preview of the asset in `cache_dir`, rendered once and then reused.
pub fn thumbnail(id: &str, cache_dir: &Path) -> AppResult<PathBuf> {
    let target = cache_dir.join(format!("{}.jpg", file_stem(id)));
    if !target.is_file() {
        std::fs::create_dir_all(cache_dir)?;
        platform::thumbnail(id, &target)?;
    }
    Ok(target)
}

/// Exports the asset into `dir` (a staging folder) and returns the written file.
pub fn export(id: &str, dir: &Path) -> AppResult<PathBuf> {
    std::fs::create_dir_all(dir)?;
    platform::export(id, dir, &file_stem(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_stem_uses_the_uuid() {
        assert_eq!(file_stem("9F2A-11/L0/001"), "photos-9F2A-11");
        assert_eq!(file_stem("plain"), "photos-plain");
    }

    #[test]
    fn recognises_exported_files() {
        let files = ["/data/media/photos-9F2A-11-1.jpg", "/x/IMG_1.jpg"];
        assert!(is_added("9F2A-11/L0/001", files.iter().copied()));
        assert!(!is_added("AB/L0/001", files.iter().copied()));
    }
}
