// Port of node-fs-extra/lib/remove/index.js: `rm -r --force` semantics
// (implemented natively, matching node's fs.rm with recursive: true, force: true).
use std::io;

use rayon::prelude::*;

use crate::errors::fserr;

fn unlink_sync(path: &str) -> io::Result<()> {
    unsafe {
        let c = std::ffi::CString::new(path.as_bytes().to_vec())
            .map_err(|_| io::Error::from_raw_os_error(libc::EINVAL))?;
        if libc::unlink(c.as_ptr()) == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

fn rmdir_sync(path: &str) -> io::Result<()> {
    unsafe {
        let c = std::ffi::CString::new(path.as_bytes().to_vec())
            .map_err(|_| io::Error::from_raw_os_error(libc::EINVAL))?;
        if libc::rmdir(c.as_ptr()) == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

/// Recursive removal, matching node's rm {recursive, force}: symlinks (even
/// dangling) are unlinked as themselves, files unlinked, dirs bottom-up.
pub fn remove_impl(path: &str) -> Result<(), napi::Error> {
    let md = match std::fs::symlink_metadata(path) {
        Ok(md) => md,
        Err(e) => {
            if e.raw_os_error() == Some(libc::ENOENT) {
                return Ok(()); // force: don't fail on non-existent paths
            }
            return Err(fserr(&e, "lstat", path));
        }
    };

    let file_type = md.file_type();
    if file_type.is_symlink() {
        return unlink_sync(path).map_err(|e| fserr(&e, "unlink", path));
    }
    if !file_type.is_dir() {
        return unlink_sync(path).map_err(|e| fserr(&e, "unlink", path));
    }

    // Directory: remove children in parallel, then the directory itself.
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => return Err(fserr(&e, "scandir", path)),
    };
    let children: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| {
            let name = e.file_name();
            format!("{}/{}", path.trim_end_matches('/'), name.to_string_lossy())
        })
        .collect();

    let errors: Vec<napi::Error> = children
        .into_par_iter()
        .filter_map(|child| remove_impl(&child).err())
        .collect();
    if let Some(first) = errors.into_iter().next() {
        return Err(first);
    }

    rmdir_sync(path).map_err(|e| fserr(&e, "rmdir", path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove() {
        let t = std::env::temp_dir().join("rustfs-remove-test");
        let _ = std::fs::remove_dir_all(&t);
        std::fs::create_dir_all(t.join("a/b/c")).unwrap();
        std::fs::write(t.join("a/b/f"), "x").unwrap();
        std::os::unix::fs::symlink("../missing", t.join("a/b/dangling")).unwrap();
        remove_impl(t.to_str().unwrap()).unwrap();
        assert!(!t.exists());
    }
}