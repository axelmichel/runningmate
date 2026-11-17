//! Smaller copies of photos for the media folder: scaled down so the longest side fits the
//! photo size setting (never up) and saved as JPEG. JPEG and PNG go through the `image` crate on
//! every system; HEIC needs macOS's `sips` and is kept as it is elsewhere. A version that isn't
//! noticeably smaller than the original is thrown away. Linked originals never come here.

use std::{
    fs,
    io::BufWriter,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use image::{
    DynamicImage, ImageDecoder, ImageEncoder, ImageReader,
    codecs::{
        jpeg::JpegEncoder,
        png::{self, PngEncoder},
    },
    imageops::FilterType,
    metadata::Orientation,
};

use crate::{
    error::{AppError, AppResult},
    settings::PhotoSize,
};

/// A smaller version of a photo, in a temporary file in the folder it is meant for.
#[derive(Debug)]
pub struct Shrunk {
    pub temp: PathBuf,
    /// "jpg", or "png" for images with transparency.
    pub extension: &'static str,
    pub bytes: u64,
}

/// Removes the file when dropped, unless it is kept.
struct TempFile(Option<PathBuf>);

impl TempFile {
    fn new(dir: &Path) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        // `sips` wants a matching suffix; the image crate doesn't care.
        Self(Some(
            dir.join(format!(".shrinking-{}-{n}.jpg", std::process::id())),
        ))
    }

    fn path(&self) -> &Path {
        self.0.as_deref().expect("temp file kept already")
    }

    fn keep(mut self) -> PathBuf {
        self.0.take().expect("temp file kept already")
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = fs::remove_file(path);
        }
    }
}

fn lowercase_extension(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// Whether this system can shrink the file: JPEG and PNG anywhere, HEIC on macOS.
pub fn supported(path: &Path) -> bool {
    let extension = lowercase_extension(path);
    matches!(extension.as_str(), "jpg" | "jpeg" | "png")
        || (extension == "heic" && cfg!(target_os = "macos"))
}

/// A shrunk photo is kept only at this fraction of the original's size or less.
const MIN_RATIO: f64 = 0.9;

/// Writes a smaller version of `source` into `dir`. `None` when the size setting keeps photos
/// as they are, the format can't be shrunk here, or the result wouldn't be noticeably smaller.
pub fn shrink(source: &Path, size: PhotoSize, dir: &Path) -> AppResult<Option<Shrunk>> {
    let Some((max_side, quality)) = size.limits() else {
        return Ok(None);
    };
    if !supported(source) {
        return Ok(None);
    }
    let original = fs::metadata(source)?.len();
    fs::create_dir_all(dir)?;
    let temp = TempFile::new(dir);
    let extension = if lowercase_extension(source) == "heic" {
        sips::convert(source, max_side, quality, temp.path())?
    } else {
        encode(source, max_side, quality, temp.path())?
    };
    let bytes = fs::metadata(temp.path())?.len();
    // Saving less than a tenth isn't worth another generation of JPEG artefacts.
    if bytes as f64 > original as f64 * MIN_RATIO {
        return Ok(None);
    }
    Ok(Some(Shrunk {
        temp: temp.keep(),
        extension,
        bytes,
    }))
}

fn image_error(error: image::ImageError) -> AppError {
    AppError::Invalid(error.to_string())
}

/// Turns the image upright as its EXIF orientation says, scales it down and saves it as JPEG
/// (PNG if it has transparency). The colour profile and the EXIF data (capture date, camera,
/// location) are kept, with the orientation reset as the pixels are upright now.
fn encode(source: &Path, max_side: u32, quality: u8, target: &Path) -> AppResult<&'static str> {
    let mut decoder = ImageReader::open(source)?
        .with_guessed_format()?
        .into_decoder()
        .map_err(image_error)?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let icc = decoder.icc_profile().ok().flatten();
    let mut exif = decoder.exif_metadata().ok().flatten();
    if let Some(exif) = exif.as_mut() {
        let _ = Orientation::remove_from_exif_chunk(exif);
    }
    let mut image = DynamicImage::from_decoder(decoder).map_err(image_error)?;
    image.apply_orientation(orientation);
    if image.width().max(image.height()) > max_side {
        image = image.resize(max_side, max_side, FilterType::Lanczos3);
    }
    let file = BufWriter::new(fs::File::create(target)?);
    if transparent(&image) {
        let mut encoder = PngEncoder::new_with_quality(
            file,
            png::CompressionType::Best,
            png::FilterType::Adaptive,
        );
        if let Some(icc) = icc {
            let _ = encoder.set_icc_profile(icc);
        }
        if let Some(exif) = exif {
            let _ = encoder.set_exif_metadata(exif);
        }
        image.write_with_encoder(encoder).map_err(image_error)?;
        Ok("png")
    } else {
        let mut encoder = JpegEncoder::new_with_quality(file, quality);
        if let Some(icc) = icc {
            let _ = encoder.set_icc_profile(icc);
        }
        if let Some(exif) = exif {
            let _ = encoder.set_exif_metadata(exif);
        }
        image.write_with_encoder(encoder).map_err(image_error)?;
        Ok("jpg")
    }
}

/// Has a pixel that isn't fully opaque (an RGBA screenshot that is opaque everywhere has none).
fn transparent(image: &DynamicImage) -> bool {
    match image {
        DynamicImage::ImageRgba8(rgba) => rgba.pixels().any(|p| p[3] < u8::MAX),
        DynamicImage::ImageLumaA8(la) => la.pixels().any(|p| p[1] < u8::MAX),
        other => other.has_alpha() && other.to_rgba8().pixels().any(|p| p[3] < u8::MAX),
    }
}

/// Longest side in pixels of each file, `None` where it can't be read (or HEIC off macOS).
pub fn longest_sides(paths: &[PathBuf]) -> Vec<Option<u32>> {
    let heic: Vec<PathBuf> = paths
        .iter()
        .filter(|p| lowercase_extension(p) == "heic")
        .cloned()
        .collect();
    let mut heic_sides = sips::longest_sides(&heic).into_iter();
    paths
        .iter()
        .map(|path| {
            if lowercase_extension(path) == "heic" {
                heic_sides.next().flatten()
            } else {
                ImageReader::open(path)
                    .ok()?
                    .with_guessed_format()
                    .ok()?
                    .into_dimensions()
                    .ok()
                    .map(|(w, h)| w.max(h))
            }
        })
        .collect()
}

#[cfg(target_os = "macos")]
mod sips {
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        process::Command,
    };

    use crate::error::{AppError, AppResult};

    const SIPS: &str = "/usr/bin/sips";

    /// Apple's JPEG quality setting for the `image` crate's (libjpeg's) `quality`: ImageIO's
    /// scale gives much larger files at the same number, so it is mapped to the one that comes
    /// out at about the same size.
    fn image_io_quality(quality: u8) -> u8 {
        quality.saturating_sub(20).max(1)
    }

    /// Converts a HEIC photo to JPEG, scaled down to `max_side` if it is larger. `sips` keeps the
    /// orientation tag and the EXIF data.
    pub fn convert(
        source: &Path,
        max_side: u32,
        quality: u8,
        target: &Path,
    ) -> AppResult<&'static str> {
        let side = longest_sides(&[source.to_path_buf()])
            .pop()
            .flatten()
            .ok_or_else(|| AppError::Invalid(format!("can't read {}", source.display())))?;
        let mut command = Command::new(SIPS);
        if side > max_side {
            command.arg("-Z").arg(max_side.to_string());
        }
        let output = command
            .args(["-s", "format", "jpeg", "-s", "formatOptions"])
            .arg(image_io_quality(quality).to_string())
            .arg(source)
            .arg("--out")
            .arg(target)
            .output()?;
        if !output.status.success() || !target.is_file() {
            return Err(AppError::Invalid(format!(
                "sips couldn't convert {}: {}",
                source.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok("jpg")
    }

    /// Longest sides from one `sips -g pixelWidth -g pixelHeight` run per 100 files. It prints
    /// each file's absolute path followed by indented `key: value` lines and skips unreadable
    /// files, so results are matched by path.
    pub fn longest_sides(paths: &[PathBuf]) -> Vec<Option<u32>> {
        let mut sides: HashMap<PathBuf, (u32, u32)> = HashMap::new();
        for chunk in paths.chunks(100) {
            let Ok(output) = Command::new(SIPS)
                .args(["-g", "pixelWidth", "-g", "pixelHeight"])
                .args(chunk)
                .output()
            else {
                continue;
            };
            let mut current: Option<PathBuf> = None;
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                if !line.starts_with(' ') {
                    current = Some(canonical(Path::new(line)));
                    continue;
                }
                let Some((key, value)) = line.trim().split_once(':') else {
                    continue;
                };
                let (Some(file), Ok(value)) = (&current, value.trim().parse::<u32>()) else {
                    continue;
                };
                let entry = sides.entry(file.clone()).or_default();
                match key {
                    "pixelWidth" => entry.0 = value,
                    "pixelHeight" => entry.1 = value,
                    _ => {}
                }
            }
        }
        paths
            .iter()
            .map(|path| {
                sides
                    .get(&canonical(path))
                    .map(|(w, h)| (*w).max(*h))
                    .filter(|side| *side > 0)
            })
            .collect()
    }

    fn canonical(path: &Path) -> PathBuf {
        std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }
}

#[cfg(not(target_os = "macos"))]
mod sips {
    use std::path::{Path, PathBuf};

    use crate::error::{AppError, AppResult};

    pub fn convert(
        source: &Path,
        _max_side: u32,
        _quality: u8,
        _target: &Path,
    ) -> AppResult<&'static str> {
        Err(AppError::Invalid(format!(
            "HEIC photos can only be converted on macOS: {}",
            source.display()
        )))
    }

    pub fn longest_sides(paths: &[PathBuf]) -> Vec<Option<u32>> {
        vec![None; paths.len()]
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use image::{Rgb, RgbImage, Rgba, RgbaImage};

    /// A noisy gradient, so JPEG has some detail to compress.
    pub fn photo(width: u32, height: u32) -> RgbImage {
        RgbImage::from_fn(width, height, |x, y| {
            let noise = ((x.wrapping_mul(7919) ^ y.wrapping_mul(104_729)) % 64) as u8;
            Rgb([
                (x * 255 / width) as u8 / 2 + noise,
                (y * 255 / height) as u8 / 2 + noise,
                128u8.wrapping_add(noise),
            ])
        })
    }

    pub fn write_jpeg(path: &Path, image: &RgbImage, quality: u8) {
        let file = BufWriter::new(fs::File::create(path).unwrap());
        JpegEncoder::new_with_quality(file, quality)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                image::ExtendedColorType::Rgb8,
            )
            .unwrap();
    }

    /// By content: temp files always end in `.jpg`.
    fn dimensions(path: &Path) -> (u32, u32) {
        ImageReader::open(path)
            .unwrap()
            .with_guessed_format()
            .unwrap()
            .into_dimensions()
            .unwrap()
    }

    const TAKEN: &str = "2024:05:01 10:00:00";

    /// A minimal EXIF block with the orientation and the date taken, as cameras write them.
    fn exif(orientation: u16) -> Vec<u8> {
        let mut exif = b"MM\0\x2a\0\0\0\x08".to_vec();
        exif.extend_from_slice(&2u16.to_be_bytes()); // two entries
        exif.extend_from_slice(&0x0112u16.to_be_bytes()); // Orientation
        exif.extend_from_slice(&3u16.to_be_bytes()); // SHORT
        exif.extend_from_slice(&1u32.to_be_bytes()); // count
        exif.extend_from_slice(&orientation.to_be_bytes());
        exif.extend_from_slice(&[0, 0]);
        exif.extend_from_slice(&0x0132u16.to_be_bytes()); // DateTime
        exif.extend_from_slice(&2u16.to_be_bytes()); // ASCII
        exif.extend_from_slice(&20u32.to_be_bytes()); // count, with the NUL
        exif.extend_from_slice(&38u32.to_be_bytes()); // offset, right after this IFD
        exif.extend_from_slice(&0u32.to_be_bytes()); // no next IFD
        exif.extend_from_slice(TAKEN.as_bytes());
        exif.push(0);
        exif
    }

    fn write_jpeg_with_exif(path: &Path, image: &RgbImage, orientation: u16) {
        let file = BufWriter::new(fs::File::create(path).unwrap());
        let mut encoder = JpegEncoder::new_with_quality(file, 98);
        encoder.set_exif_metadata(exif(orientation)).unwrap();
        encoder
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                image::ExtendedColorType::Rgb8,
            )
            .unwrap();
    }

    fn keeps_date_taken(path: &Path) -> bool {
        let bytes = fs::read(path).unwrap();
        bytes.windows(TAKEN.len()).any(|w| w == TAKEN.as_bytes())
    }

    #[test]
    fn scales_the_longest_side_down_and_saves_jpeg() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("wide.png");
        DynamicImage::ImageRgb8(photo(3000, 1500))
            .save(&source)
            .unwrap();
        let shrunk = shrink(&source, PhotoSize::Small, dir.path())
            .unwrap()
            .unwrap();
        assert_eq!(shrunk.extension, "jpg");
        assert_eq!(dimensions(&shrunk.temp), (1920, 960));
        assert_eq!(shrunk.bytes, fs::metadata(&shrunk.temp).unwrap().len());
        assert!(shrunk.bytes < fs::metadata(&source).unwrap().len());
        assert_eq!(
            ImageReader::open(&shrunk.temp)
                .unwrap()
                .with_guessed_format()
                .unwrap()
                .format(),
            Some(image::ImageFormat::Jpeg)
        );
    }

    #[test]
    fn never_scales_up_and_a_lower_quality_makes_files_smaller() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("small.jpg");
        write_jpeg(&source, &photo(800, 600), 100);
        let shrunk = shrink(&source, PhotoSize::Small, dir.path())
            .unwrap()
            .unwrap();
        assert_eq!(dimensions(&shrunk.temp), (800, 600));
        assert!(shrunk.bytes < fs::metadata(&source).unwrap().len());
    }

    #[test]
    fn keeps_the_original_when_it_is_not_smaller() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("tight.jpg");
        write_jpeg(&source, &photo(800, 600), 40);
        assert!(
            shrink(&source, PhotoSize::Large, dir.path())
                .unwrap()
                .is_none()
        );
        assert!(
            shrink(&source, PhotoSize::Original, dir.path())
                .unwrap()
                .is_none()
        );
        // Only the original is left; the attempt's temp file is gone.
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn applies_the_exif_orientation_before_scaling() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("portrait.jpg");
        // Stored landscape, shown portrait (rotated 90° clockwise).
        write_jpeg_with_exif(&source, &photo(4000, 3000), 6);

        let shrunk = shrink(&source, PhotoSize::Recommended, dir.path())
            .unwrap()
            .unwrap();
        assert_eq!(dimensions(&shrunk.temp), (1920, 2560));
        let mut decoder = ImageReader::open(&shrunk.temp)
            .unwrap()
            .with_guessed_format()
            .unwrap()
            .into_decoder()
            .unwrap();
        // The EXIF data is kept, but says "upright" now.
        assert!(decoder.exif_metadata().unwrap().is_some());
        assert_eq!(decoder.orientation().unwrap(), Orientation::NoTransforms);
        assert!(keeps_date_taken(&shrunk.temp));
    }

    #[test]
    fn transparent_png_stays_png() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("logo.png");
        RgbaImage::from_fn(3000, 2000, |x, y| {
            let noise = ((x * 31 + y * 17) % 200) as u8;
            Rgba([noise, 255 - noise, 90, if x < 1500 { 0 } else { 255 }])
        })
        .save(&source)
        .unwrap();
        let shrunk = shrink(&source, PhotoSize::Small, dir.path())
            .unwrap()
            .unwrap();
        assert_eq!(shrunk.extension, "png");
        assert_eq!(dimensions(&shrunk.temp), (1920, 1280));
    }

    #[test]
    fn reads_the_longest_side() {
        let dir = tempfile::tempdir().unwrap();
        let tall = dir.path().join("tall.jpg");
        write_jpeg(&tall, &photo(300, 500), 80);
        let broken = dir.path().join("broken.jpg");
        fs::write(&broken, b"not a photo").unwrap();
        assert_eq!(longest_sides(&[tall, broken]), [Some(500), None]);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn converts_heic_with_sips() {
        let dir = tempfile::tempdir().unwrap();
        let jpeg = dir.path().join("source.jpg");
        write_jpeg_with_exif(&jpeg, &photo(3000, 2000), 6);
        let heic = dir.path().join("photo.heic");
        let status = std::process::Command::new("/usr/bin/sips")
            .args(["-s", "format", "heic", "-s", "formatOptions", "best"])
            .arg(&jpeg)
            .arg("--out")
            .arg(&heic)
            .output()
            .unwrap();
        assert!(status.status.success());
        assert_eq!(longest_sides(std::slice::from_ref(&heic)), [Some(3000)]);

        let shrunk = shrink(&heic, PhotoSize::Small, dir.path())
            .unwrap()
            .unwrap();
        assert_eq!(shrunk.extension, "jpg");
        assert_eq!(dimensions(&shrunk.temp), (1920, 1280));
        assert!(keeps_date_taken(&heic));
        assert!(keeps_date_taken(&shrunk.temp));
        // `sips` keeps the orientation tag ("rotate 90°") instead of turning the pixels.
        let orientation = ImageReader::open(&shrunk.temp)
            .unwrap()
            .with_guessed_format()
            .unwrap()
            .into_decoder()
            .unwrap()
            .orientation()
            .unwrap();
        assert_eq!(orientation, Orientation::Rotate90);
    }
}
