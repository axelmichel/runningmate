//! `{file_id}.tcx.tar.gz` archives in `~/RunningData/imports`, as written by
//! `TcxFileImporter.archive_file`.

use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use flate2::{Compression, read::GzDecoder, write::GzEncoder};

use crate::error::{AppError, AppResult};

pub fn archive_path(imports_dir: &Path, file_id: &str) -> PathBuf {
    imports_dir.join(format!("{file_id}.tcx.tar.gz"))
}

/// Writes `xml` as `{file_id}.tcx` into `{file_id}.tcx.tar.gz`, replacing an existing archive.
pub fn write(imports_dir: &Path, file_id: &str, xml: &str) -> AppResult<PathBuf> {
    std::fs::create_dir_all(imports_dir)?;
    let path = archive_path(imports_dir, file_id);
    let partial = path.with_extension("gz.partial");
    {
        let encoder = GzEncoder::new(File::create(&partial)?, Compression::default());
        let mut tar = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(xml.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(chrono::Utc::now().timestamp().max(0) as u64);
        header.set_cksum();
        tar.append_data(&mut header, format!("{file_id}.tcx"), xml.as_bytes())?;
        tar.into_inner()?.finish()?;
    }
    std::fs::rename(&partial, &path)?;
    Ok(path)
}

/// Reads the TCX file inside an archive.
pub fn read(path: &Path) -> AppResult<String> {
    let mut tar = tar::Archive::new(GzDecoder::new(File::open(path)?));
    for entry in tar.entries()? {
        let mut entry = entry?;
        let is_tcx = entry
            .path()?
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("tcx"));
        if is_tcx {
            let mut xml = String::new();
            entry.read_to_string(&mut xml)?;
            return Ok(xml);
        }
    }
    Err(AppError::NotFound(format!(
        "TCX file in {}",
        path.display()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(dir.path(), "2025-05-06_run", "<xml/>").unwrap();
        assert!(path.ends_with("2025-05-06_run.tcx.tar.gz"));
        assert_eq!(read(&path).unwrap(), "<xml/>");
    }
}
