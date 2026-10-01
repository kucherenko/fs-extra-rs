// Port of node-fs-extra/lib/ensure/{file,link,symlink}.js and
// symlink-{paths,type}.js — "ensure" methods that create files, hard links
// and symlinks idempotently.
use std::io;

use crate::errors::{fserr, fserr_dual, plain_error, FsFail};
use crate::mkdirs::make_dir_sync_impl;
use crate::paths::{dirname, is_absolute, join2, relative};
use crate::util_stat::{are_identical, lstat, stat, St};

/// Low-level: write data to a file like node fs.writeFile (flag 'w').
pub(crate) fn write_file_sync_impl(
    path: &str,
    data: &[u8],
    mode: u32,
    flag: &str,
) -> Result<(), napi::Error> {
    let flags = open_flags(flag, path)?;
    let cpath = cstr(path, "open")?;
    let fd = unsafe { libc::open(cpath.as_ptr(), flags, (mode & 0o7777) as libc::c_int) };
    if fd < 0 {
        let e = io::Error::last_os_error();
        return Err(fserr(&e, "open", path));
    }
    let mut written = 0usize;
    while written < data.len() {
        let n = unsafe { libc::write(fd, data[written..].as_ptr().cast(), data.len() - written) };
        if n < 0 {
            let e = io::Error::last_os_error();
            unsafe { libc::close(fd) };
            return Err(fserr(&e, "write", path));
        }
        written += n as usize;
    }
    let r = unsafe { libc::close(fd) };
    if r != 0 {
        let e = io::Error::last_os_error();
        return Err(fserr(&e, "close", path));
    }
    Ok(())
}

fn cstr(p: &str, syscall: &str) -> Result<std::ffi::CString, napi::Error> {
    std::ffi::CString::new(p.as_bytes().to_vec()).map_err(|_e| {
        let f = FsFail::new(&io::Error::from_raw_os_error(libc::EINVAL), syscall, p);
        f.to_js_error(f.single_message())
    })
}

/// Map node's open flag strings to libc flags (subset used by the port).
pub(crate) fn open_flags(flag: &str, path: &str) -> Result<i32, napi::Error> {
    let base: i32 = match flag {
        "w" => (libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC) as i32,
        "wx" => (libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC | libc::O_EXCL) as i32,
        "a" => (libc::O_WRONLY | libc::O_CREAT | libc::O_APPEND) as i32,
        "ax" => (libc::O_WRONLY | libc::O_CREAT | libc::O_APPEND | libc::O_EXCL) as i32,
        "r+" => libc::O_RDWR as i32,
        "w+" => (libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC) as i32,
        "wx+" => (libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC | libc::O_EXCL) as i32,
        "a+" => (libc::O_RDWR | libc::O_CREAT | libc::O_APPEND) as i32,
        "ax+" => (libc::O_RDWR | libc::O_CREAT | libc::O_APPEND | libc::O_EXCL) as i32,
        other => {
            return Err(plain_error(format!(
                "The value of \"flags\" is invalid: {}. Received the value {}",
                path, other
            )));
        }
    };
    Ok(base)
}

// Port of lib/ensure/file.js createFile/createFileSync: one shared core (the
// source's async and sync forms do the same operations in the same order).
pub fn create_file_sync_impl(file: &str) -> Result<(), napi::Error> {
    create_file(file)
}

fn create_file(file: &str) -> Result<(), napi::Error> {
    let stats = std::fs::metadata(file).ok().map(|m| St::from_meta(&m));
    if let Some(stats) = stats {
        if stats.is_file() {
            return Ok(());
        }
    }

    let dir = dirname(file);
    match stat(&dir) {
        Err(err) => {
            if err.raw_os_error() == Some(libc::ENOENT) {
                // if the directory doesn't exist, make it
                make_dir_sync_impl(&dir, None)?;
                write_file_sync_impl(file, b"", 0o666, "w")
            } else {
                Err(fserr(&err, "stat", &dir))
            }
        }
        Ok(dir_stats) => {
            if dir_stats.is_dir() {
                write_file_sync_impl(file, b"", 0o666, "w")
            } else {
                // parent is not a directory. This is just to cause an internal
                // ENOTDIR error to be thrown, like the source does with readdir
                match std::fs::read_dir(&dir) {
                    Err(err) => Err(fserr(&err, "scandir", &dir)),
                    Ok(_) => Ok(()),
                }
            }
        }
    }
}

/// A plain `new Error('...')`-shaped failure.
fn srcpath_not_exist(msg: &str) -> napi::Error {
    plain_error(msg)
}

// Port of lib/ensure/symlink-paths.js symlinkPathsSync
pub fn symlink_paths_sync(srcpath: &str, dstpath: &str) -> Result<(String, String), napi::Error> {
    if is_absolute(srcpath) {
        if std::fs::symlink_metadata(srcpath).is_err() {
            return Err(srcpath_not_exist("absolute srcpath does not exist"));
        }
        return Ok((srcpath.to_string(), srcpath.to_string()));
    }

    let dstdir = dirname(dstpath);
    let relative_to_dst = join2(&dstdir, srcpath);

    if std::fs::symlink_metadata(&relative_to_dst).is_ok() {
        return Ok((relative_to_dst, srcpath.to_string()));
    }

    if std::fs::symlink_metadata(srcpath).is_err() {
        return Err(srcpath_not_exist("relative srcpath does not exist"));
    }

    Ok((srcpath.to_string(), relative(&dstdir, srcpath)))
}

// Port of lib/ensure/symlink-type.js symlinkTypeSync
pub fn symlink_type_sync(srcpath: &str, _type: Option<&str>) -> Result<String, napi::Error> {
    if let Some(t) = _type {
        if !t.is_empty() {
            return Ok(t.to_string());
        }
    }
    match lstat(srcpath) {
        Err(_) => Ok("file".to_string()),
        Ok(stats) => Ok(if stats.is_dir() { "dir" } else { "file" }.to_string()),
    }
}

// Port of lib/ensure/link.js createLinkSync
pub fn create_link_sync_impl(srcpath: &str, dstpath: &str) -> Result<(), napi::Error> {
    let dst_stat = lstat(dstpath).ok();

    let src_stat = match lstat(srcpath) {
        Err(err) => {
            // err.message = err.message.replace('lstat', 'ensureLink')
            let mut f = FsFail::new(&err, "lstat", srcpath);
            f.syscall = "ensureLink".to_string();
            let msg = f.single_message();
            return Err(f.to_js_error(msg));
        }
        Ok(s) => s,
    };

    if let Some(dst_stat) = dst_stat {
        if are_identical(&src_stat, &dst_stat) {
            return Ok(());
        }
    }

    let dir = dirname(dstpath);
    let dir_exists = std::fs::metadata(&dir).is_ok(); // graceful fs.existsSync
    if !dir_exists {
        make_dir_sync_impl(&dir, None)?;
    }

    link_sync(srcpath, dstpath)
}

fn link_sync(srcpath: &str, dstpath: &str) -> Result<(), napi::Error> {
    let csrc = std::ffi::CString::new(srcpath.as_bytes().to_vec())
        .map_err(|_| io::Error::from_raw_os_error(libc::EINVAL))?;
    let cdst = std::ffi::CString::new(dstpath.as_bytes().to_vec())
        .map_err(|_| io::Error::from_raw_os_error(libc::EINVAL))?;
    let rc = unsafe { libc::link(csrc.as_ptr(), cdst.as_ptr()) };
    if rc == 0 {
        Ok(())
    } else {
        let e = io::Error::last_os_error();
        Err(fserr_dual(&e, "link", srcpath, dstpath))
    }
}

pub(crate) fn symlink_sync(srcpath: &str, dstpath: &str, _type: &str) -> Result<(), napi::Error> {
    #[cfg(windows)]
    {
        let src = std::path::Path::new(srcpath);
        let dst = std::path::Path::new(dstpath);
        let target_meta = std::fs::metadata(srcpath);
        let is_dir = _type == "dir" || _type == "junction"
            || target_meta.map(|m| m.is_dir()).unwrap_or(false);
        let r = if is_dir {
            std::os::windows::fs::symlink_dir(src, dst)
        } else {
            std::os::windows::fs::symlink_file(src, dst)
        };
        return match r {
            Ok(()) => Ok(()),
            Err(e) => Err(fserr_dual(&e, "symlink", dstpath, srcpath)),
        };
    }
    #[cfg(unix)]
    {
        let csrc = std::ffi::CString::new(srcpath.as_bytes().to_vec())
            .map_err(|_| io::Error::from_raw_os_error(libc::EINVAL));
        let cdst = std::ffi::CString::new(dstpath.as_bytes().to_vec())
            .map_err(|_| io::Error::from_raw_os_error(libc::EINVAL));
        let (csrc, cdst) = match (csrc, cdst) {
            (Ok(a), Ok(b)) => (a, b),
            _ => return Err(plain_error(format!("EINVAL: invalid argument, symlink '{}' -> '{}'", dstpath, srcpath))),
        };
        let rc = unsafe { libc::symlink(csrc.as_ptr(), cdst.as_ptr()) as i32 };
        if rc == 0 {
            Ok(())
        } else {
            let e = io::Error::last_os_error();
            Err(fserr_dual(&e, "symlink", dstpath, srcpath))
        }
    }
}

// Port of lib/ensure/symlink.js createSymlinkSync
pub fn create_symlink_sync_impl(
    srcpath: &str,
    dstpath: &str,
    _type: Option<&str>,
) -> Result<(), napi::Error> {
    let stats = std::fs::symlink_metadata(dstpath).ok().map(|m| St::from_meta(&m));
    if let Some(stats) = stats {
        if stats.is_symlink() {
            // When srcpath is relative, resolve it relative to dstpath's
            // directory (standard symlink behavior) or fall back to cwd if
            // that doesn't exist
            let src_stat = if is_absolute(srcpath) {
                match stat(srcpath) {
                    Ok(s) => s,
                    Err(e) => return Err(fserr(&e, "stat", srcpath)),
                }
            } else {
                let dstdir = dirname(dstpath);
                let relative_to_dst = join2(&dstdir, &srcpath.to_string());
                match stat(&relative_to_dst) {
                    Ok(s) => s,
                    Err(_) => match stat(srcpath) {
                        Ok(s) => s,
                        Err(e) => return Err(fserr(&e, "stat", srcpath)),
                    },
                }
            };

            // Following the existing symlink fails with ENOENT if it's broken;
            // in that case fall through so fs.symlink reports EEXIST, rather
            // than leaking the stat ENOENT. Any other error is unexpected.
            let dst_stat = match stat(dstpath) {
                Ok(s) => Some(s),
                Err(err) => {
                    if err.raw_os_error() != Some(libc::ENOENT) {
                        return Err(fserr(&err, "stat", dstpath));
                    }
                    None
                }
            };
            if let Some(dst_stat) = dst_stat {
                if are_identical(&src_stat, &dst_stat) {
                    return Ok(());
                }
            }
        }
    }

    let (to_cwd, to_dst) = symlink_paths_sync(srcpath, dstpath)?;
    let srcpath = &to_dst;
    let _type = symlink_type_sync(&to_cwd, _type)?;
    let dir = dirname(dstpath);
    let exists = std::fs::metadata(&dir).is_ok(); // graceful fs.existsSync
    if exists {
        return symlink_sync(srcpath, dstpath, &_type);
    }
    make_dir_sync_impl(&dir, None)?;
    symlink_sync(srcpath, dstpath, &_type)
}


#[cfg(test)]
mod tests {
    // the symlink-path resolution is cwd-relative in these tests; serialize
    // the chdir-ing tests because cargo runs tests in parallel threads
    static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    use super::*;

    #[test]
    fn test_symlink_paths_sync() {
        let t = std::env::temp_dir().join("rustfs-symtest");
        let _ = std::fs::remove_dir_all(&t);
        std::fs::create_dir_all(t.join("sub")).unwrap();
        std::fs::write(t.join("foo.txt"), "x").unwrap();
        // relative paths resolve against the cwd, like the source (chdir in)
        let _guard = CWD_LOCK.lock().unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&t).unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // relativeToDst ('sub/foo.txt') does not exist; srcpath exists in
            // the cwd, per the source's table: toCwd is the cwd-relative
            // srcpath, toDst the dst-dir-relative one
            let (to_cwd, to_dst) = symlink_paths_sync("foo.txt", "sub/symlink.txt").unwrap();
            assert_eq!(to_cwd, "foo.txt", "cwd");
            assert_eq!(to_dst, "../foo.txt", "dst");
            // dstdir == '.' resolves relativeToDst = srcpath itself
            let (to_cwd, to_dst) = symlink_paths_sync("foo.txt", "symlink.txt").unwrap();
            assert_eq!(to_cwd, "foo.txt", "cwd 2");
            assert_eq!(to_dst, "foo.txt", "dst 2");
            // a file inside sub that exists as relativeToDst wins
            std::fs::write("sub/in-sub.txt", "x").unwrap();
            let (to_cwd, to_dst) = symlink_paths_sync("in-sub.txt", "sub/symlink.txt").unwrap();
            assert_eq!(to_cwd, "sub/in-sub.txt", "cwd 3");
            assert_eq!(to_dst, "in-sub.txt", "dst 3");
            // missing relative source
            let e = symlink_paths_sync("missing.txt", "sub/symlink.txt").unwrap_err();
            assert!(format!("{:?}", e).contains("relative srcpath does not exist"), "{}", e);
            // missing absolute source
            let e = symlink_paths_sync(t.join("missing-abs").to_str().unwrap(), "sub/symlink.txt").unwrap_err();
            assert!(format!("{:?}", e).contains("absolute srcpath does not exist"), "{}", e);
        }));
        std::env::set_current_dir(prev).unwrap();
        result.unwrap();
        let _ = std::fs::remove_dir_all(&t);
    }

    #[test]
    fn test_ensure_symlink_sync() {
        let t = std::env::temp_dir().join("rustfs-ensymtest");
        let _ = std::fs::remove_dir_all(&t);
        std::fs::create_dir_all(t.join("dstdir")).unwrap();
        std::fs::write(t.join("src.txt"), "x").unwrap();
        // relative srcpath resolves through the cwd ('../src.txt' from dstdir)
        let _guard = CWD_LOCK.lock().unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&t).unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            create_symlink_sync_impl("src.txt", "dstdir/lnk", None).unwrap();
            let target = std::fs::read_link(t.join("dstdir/lnk")).unwrap();
            assert_eq!(target.to_string_lossy(), "../src.txt");
            // idempotent (same relative source again)
            create_symlink_sync_impl("src.txt", "dstdir/lnk", None).unwrap();
            // existing identical link: no-op
            create_symlink_sync_impl("../src.txt", "dstdir/lnk", None).unwrap();
        }));
        std::env::set_current_dir(prev).unwrap();
        result.unwrap();
        let _ = std::fs::remove_dir_all(&t);
    }
}
// Port of lib/ensure/symlink-paths.js symlinkPaths (async form): identical
// path logic but lstat errors carry 'ensureSymlink' in place of 'lstat'.
pub fn symlink_paths_async_impl(srcpath: &str, dstpath: &str) -> Result<(String, String), napi::Error> {
    if crate::paths::is_absolute(srcpath) {
        match std::fs::symlink_metadata(srcpath) {
            Ok(_) => return Ok((srcpath.to_string(), srcpath.to_string())),
            Err(err) => {
                let mut f = FsFail::new(&err, "lstat", srcpath);
                f.syscall = "ensureSymlink".to_string();
                let msg = f.single_message();
                return Err(f.to_js_error(msg));
            }
        }
    }

    let dstdir = dirname(dstpath);
    let relative_to_dst = join2(&dstdir, srcpath);

    if std::fs::symlink_metadata(&relative_to_dst).is_ok() {
        return Ok((relative_to_dst, srcpath.to_string()));
    }

    match std::fs::symlink_metadata(srcpath) {
        Err(err) => {
            let mut f = FsFail::new(&err, "lstat", srcpath);
            f.syscall = "ensureSymlink".to_string();
            let msg = f.single_message();
            Err(f.to_js_error(msg))
        }
        Ok(_) => Ok((srcpath.to_string(), relative(&dstdir, srcpath))),
    }
}
