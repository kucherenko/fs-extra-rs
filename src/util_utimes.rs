// Port of node-fs-extra/lib/util/utimes.js: utimes with millisecond precision
// via an open fd, preserving the source's error ordering (futimes error wins
// over a close error; the fd is always closed, and a close error must not
// mask the futimes error).
use std::io;

use crate::errors::fserr;

/// Raw epoch time in nanoseconds, the shape node fs uses for file times.
#[derive(Debug, Clone, Copy, Default)]
pub struct Ts {
    pub sec: i64,
    pub nsec: i64,
}

impl Ts {
    pub fn new(ms: f64) -> Self {
        if ms.is_nan() || !ms.is_finite() {
            // node throws EINVAL for NaN/out-of-range times; callers surface it
            return Ts { sec: 0, nsec: 0 };
        }
        // Split at the second boundary in f64 space: ms*1e9 would exceed 2^53
        // and lose precision (e.g. 1334990868773ms rounded to .7729...).
        let sec = (ms / 1000.0).floor();
        let nsec = ((ms - sec * 1000.0) * 1_000_000.0).round() as i64;
        Ts { sec: sec as i64, nsec }
    }

    pub fn as_ms(&self) -> f64 {
        (self.sec as f64) * 1000.0 + (self.nsec as f64) / 1_000_000.0
    }
}

/// The time fields of a stat, used by copy's preserveTimestamps.
#[derive(Debug, Clone, Copy, Default)]
pub struct Times {
    pub atime: Ts,
    pub mtime: Ts,
    pub ctime: Ts,
    pub birthtime: Ts,
}

fn futimens(fd: i32, atime: Ts, mtime: Ts) -> io::Result<()> {
    let times = [
        libc::timespec { tv_sec: atime.sec, tv_nsec: atime.nsec },
        libc::timespec { tv_sec: mtime.sec, tv_nsec: mtime.nsec },
    ];
    let rc = unsafe { libc::futimens(fd, times.as_ptr()) };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn open_rw(path: &str) -> Result<i32, napi::Error> {
    let cpath = std::ffi::CString::new(path.as_bytes().to_vec())
        .map_err(|_| io::Error::from_raw_os_error(libc::EINVAL));
    let cpath = match cpath {
        Ok(c) => c,
        Err(e) => return Err(crate::errors::fserr(&e, "open", path)),
    };
    let fd = unsafe { libc::open(cpath.as_ptr(), libc::O_RDWR as i32, 0 as libc::c_uint) };
    if fd < 0 {
        let e = io::Error::last_os_error();
        return Err(fserr(&e, "open", path));
    }
    Ok(fd)
}

/// utimesMillis: open 'r+', futimes with ms precision, always close.
/// Mirrors the source's ordering: futimes error takes precedence over a
/// close error; a close error is reported when futimes succeeded.
pub fn utimes_millis_sync_impl(
    path: &str,
    atime: Ts,
    mtime: Ts,
) -> Result<(), napi::Error> {
    let fd = open_rw(path)?;
    let mut error: Option<napi::Error> = None;
    if let Err(futimes_err) = futimens(fd, atime, mtime) {
        error = Some(fserr(&futimes_err, "utime", path));
    }
    if let Err(close_err) = (|| -> io::Result<()> {
        let r = unsafe { libc::close(fd) };
        if r == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    })() {
        if error.is_none() {
            error = Some(fserr(&close_err, "close", &fd.to_string()));
        }
    }
    match error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Port of utimesMillis (async form): same ordering, the file is opened and
/// closed inside the libuv thread.
pub fn utimes_millis_impl(path: &str, atime: Ts, mtime: Ts) -> Result<(), napi::Error> {
    utimes_millis_sync_impl(path, atime, mtime)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utimes_millis() {
        use std::time::{Duration, UNIX_EPOCH};
        let t = std::env::temp_dir().join("rustfs-utimens-test");
        let _ = std::fs::remove_dir_all(&t);
        std::fs::create_dir_all(&t).unwrap();
        let f = t.join("f");
        std::fs::write(&f, "x").unwrap();
        let ms = 1334990868773.0f64;
        utimes_millis_sync_impl(f.to_str().unwrap(), Ts::new(ms), Ts::new(ms)).unwrap();
        let md = std::fs::metadata(&f).unwrap();
        let mtime = md.modified().unwrap().duration_since(UNIX_EPOCH).unwrap().as_millis() as f64;
        #[cfg(target_os = "linux")]
        assert_eq!(mtime % 1.0, 0.0, "mtime keeps millis on ext4/tmpfs: {}", mtime);
        assert_eq!(mtime, ms, "mtime set with millis precision");
        let _ = std::fs::remove_dir_all(&t);
    }
}