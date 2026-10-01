// Port of node-fs-extra/lib/mkdirs/make-dir.js + utils.js: recursive mkdir
// with node fs.mkdir({recursive}) semantics (first-created path returned,
// mode applied to every created directory, existing dirs tolerated, existing
// files rejected with EEXIST / ENOTDIR just like node).
use std::io;

use crate::errors::{fserr, plain_error};

/// Port of mkdirs/utils.js checkPath (win32 only; a no-op on POSIX).
fn check_path(pth: &str) -> Result<(), napi::Error> {
    #[cfg(target_os = "windows")]
    {
        use crate::paths::root_of;
        let root = root_of(pth);
        let rest = &pth[root.len()..];
        if rest.chars().any(|c| matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*')) {
            // error.code = 'EINVAL' from the source's checkPath
            return Err(napi::Error::new(napi::Status::GenericFailure, crate::errors::envelope(&format!("EINVAL: invalid argument, Path contains invalid characters: {}", pth))));
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = pth;
    }
    Ok(())
}

fn get_mode(options: Option<i32>) -> u32 {
    match options {
        Some(m) if m >= 0 => m as u32,
        Some(m) => (m as u32) & 0o7777777,
        None => 0o777,
    }
}

#[cfg(target_os = "windows")]
fn cast_mode(m: u32) -> libc::c_uint {
    m
}

#[cfg(not(target_os = "windows"))]
fn cast_mode(m: u32) -> libc::mode_t {
    m as libc::mode_t
}

/// The recursive mkdir walk. Returns the first directory it created
/// (as node does) or Ok(None) when the directory already existed.
fn mkdir_recursive(dir: &str, mode: u32) -> Result<Option<String>, napi::Error> {
    let bytes = dir.as_bytes();
    if bytes.is_empty() {
        return Err(plain_error(
            "The \"path\" argument must be of type string or an instance of Buffer or URL. Received ''",
        ));
    }

    let root_len = if bytes[0] == b'/' { 1 } else { 0 };
    // One reusable buffer with a spare NUL byte: for every level the delimiter
    // (or the spare NUL at the very end) is swapped out for the terminator,
    // so each level is two byte writes with no reallocation.
    let mut buf: Vec<u8> = bytes.to_vec();
    buf.push(0); // spare terminator slot for the final segment
    let mut first_created: Option<String> = None;
    let mut last_created = false;
    let mut end = root_len.min(bytes.len());

    while end < bytes.len() {
        // find the next non-empty segment (node skips empty segments)
        let mut seg_start = end;
        while seg_start < bytes.len() && bytes[seg_start] == b'/' {
            seg_start += 1;
        }
        if seg_start >= bytes.len() {
            break; // only trailing separators left
        }
        let mut seg_end = seg_start;
        while seg_end < bytes.len() && bytes[seg_end] != b'/' {
            seg_end += 1;
        }
        end = seg_end;
        last_created = false;

        let saved = buf[end]; // '/' or the spare NUL
        buf[end] = 0;
        let rc = unsafe { libc::mkdir(buf.as_ptr().cast(), cast_mode(mode)) };
        buf[end] = saved;
        if rc == 0 {
            last_created = true;
            if first_created.is_none() {
                first_created = Some(dir[..end].to_string());
            }
        } else {
            let e = io::Error::last_os_error();
            if e.raw_os_error() == Some(libc::EEXIST) {
                // node keeps walking on an existing component; a file in the
                // middle surfaces as ENOTDIR from the deeper mkdir, a file at
                // the end as EEXIST (dir check first, like node does)
                let is_final = bytes[end..].iter().all(|&b| b == b'/');
                if is_final {
                    let md = std::fs::metadata(&dir[..end]);
                    match md {
                        Ok(md) => {
                            if !md.is_dir() {
                                return Err(fserr(&e, "mkdir", &dir[..end]));
                            }
                        }
                        Err(se) => {
                            return Err(fserr(&se, "lstat", &dir[..end]));
                        }
                    }
                }
            } else {
                return Err(fserr(&e, "mkdir", &dir[..end]));
            }
        }
    }

    if last_created && dir.ends_with('/') {
        // node preserves the input's trailing separators on a created final
        // segment (mkdir('x/ts/', recursive) resolves to 'ts')
        if let Some(mut first) = first_created {
            first.push_str(&dir[end..]);
            return Ok(Some(first));
        }
    }
    Ok(first_created)
}

pub fn make_dir_sync_impl(dir: &str, mode: Option<i32>) -> Result<Option<String>, napi::Error> {
    check_path(dir)?;
    mkdir_recursive(dir, get_mode(mode))
}

pub fn make_dir_impl(dir: &str, mode: Option<i32>) -> Result<(), napi::Error> {
    make_dir_sync_impl(dir, mode).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mkdir_recursive() {
        let t = std::env::temp_dir().join("rustfs-mkdir-test");
        let _ = std::fs::remove_dir_all(&t);
        // the temp root must pre-exist so the walk creates a/b/c only (node
        // returns the FIRST created path, which would otherwise be t itself)
        std::fs::create_dir_all(&t).unwrap();
        let dir = t.join("a/b/c");
        assert!(!dir.exists());
        let first = make_dir_sync_impl(dir.to_str().unwrap(), None).unwrap();
        // node returns the FIRST created path: 'a', not the leaf
        assert_eq!(first.as_deref(), Some(t.join("a").to_str().unwrap()));
        let again = make_dir_sync_impl(dir.to_str().unwrap(), None).unwrap();
        assert_eq!(again, None);
        assert!(dir.is_dir());
        std::fs::write(t.join("file-blocker"), "x").unwrap();
        let e = make_dir_sync_impl(t.join("file-blocker/x").to_str().unwrap(), None).unwrap_err();
        assert!(e.to_string().contains("ENOTDIR"), "{}", e);
        let e = make_dir_sync_impl(t.join("file-blocker").to_str().unwrap(), None).unwrap_err();
        assert!(e.to_string().contains("EEXIST"), "{}", e);
        let _ = std::fs::remove_dir_all(&t);
    }
}