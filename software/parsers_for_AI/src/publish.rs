use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct PublishError {
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directory(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "atp_publish_{}_{}_{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn replaces_complete_document_and_leaves_no_temporary_file() {
        let dir = directory("replace");
        let output = dir.join("out.md");
        std::fs::write(&output, "old report").unwrap();
        atomic_write(&output, "new report\nЮникод\n").unwrap();
        assert_eq!(
            std::fs::read_to_string(&output).unwrap(),
            "new report\nЮникод\n"
        );
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn failed_replace_preserves_existing_report_and_cleans_temp() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = directory("locked");
        let output = dir.join("out.md");
        std::fs::write(&output, "old report").unwrap();
        let held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&output)
            .unwrap();
        assert!(atomic_write(&output, "new report").is_err());
        assert_eq!(std::fs::read_to_string(&output).unwrap(), "old report");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        drop(held);
        std::fs::remove_dir_all(dir).unwrap();
    }
}

pub fn atomic_write(path: &Path, content: &str) -> Result<(), PublishError> {
    let dir = match path.parent() {
        Some(d) if !d.as_os_str().is_empty() => d.to_path_buf(),
        _ => PathBuf::from("."),
    };
    std::fs::create_dir_all(&dir).map_err(|e| PublishError {
        message: format!("cannot create output directory '{}': {}", dir.display(), e),
    })?;

    let fname = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "output.md".to_string());
    let tmp = dir.join(format!(
        ".{}.tmp.{}.{}",
        fname,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    let write_result = (|| -> io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        Ok(())
    })();

    if let Err(e) = write_result {
        let _ = std::fs::remove_file(&tmp);
        return Err(PublishError {
            message: format!("cannot write temporary file '{}': {}", tmp.display(), e),
        });
    }

    let written_len = std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);
    if written_len != content.len() as u64 {
        let _ = std::fs::remove_file(&tmp);
        return Err(PublishError {
            message: format!(
                "validation failed: wrote {} bytes, expected {}",
                written_len,
                content.len()
            ),
        });
    }

    if let Err(e) = replace_file(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(PublishError {
            message: format!("cannot publish '{}': {}", path.display(), e),
        });
    }
    Ok(())
}

fn replace_file(tmp: &Path, dst: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Foundation::TRUE;
        use windows_sys::Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        };
        let tmp_wide: Vec<u16> = tmp.as_os_str().encode_wide().chain(Some(0)).collect();
        let dst_wide: Vec<u16> = dst.as_os_str().encode_wide().chain(Some(0)).collect();
        let ok = unsafe {
            MoveFileExW(
                tmp_wide.as_ptr(),
                dst_wide.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if ok != TRUE {
            let err = io::Error::last_os_error();
            return Err(err);
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(tmp, dst)
    }
}
