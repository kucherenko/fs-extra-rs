// Node-style fs error construction: identical code/errno/syscall/message format.
use std::io;

use napi::{Error, Status};

/// Summary strings mirror uv_strerror (lowercase), which is what the messages
/// of Node.js fs errors are built from.
fn errno_summary(errno: i32) -> String {
    let fixed: &(i32, &'static str) = &match errno {
        libc::EPERM => (0, "operation not permitted"),
        libc::ENOENT => (0, "no such file or directory"),
        libc::ESRCH => (0, "no such process"),
        libc::EINTR => (0, "interrupted system call"),
        libc::EIO => (0, "input/output error"),
        libc::ENXIO => (0, "no such device or address"),
        libc::E2BIG => (0, "argument list too long"),
        libc::EBADF => (0, "bad file descriptor"),
        libc::EAGAIN => (0, "resource temporarily unavailable"),
        libc::ENOMEM => (0, "cannot allocate memory"),
        libc::EACCES => (0, "permission denied"),
        libc::EFAULT => (0, "bad address in system call argument"),
        libc::EBUSY => (0, "resource busy or locked"),
        libc::EEXIST => (0, "file already exists"),
        libc::EXDEV => (0, "invalid cross-device link"),
        libc::ENODEV => (0, "no such device"),
        libc::ENOTDIR => (0, "not a directory"),
        libc::EISDIR => (0, "illegal operation on a directory"),
        libc::EINVAL => (0, "invalid argument"),
        libc::ENFILE => (0, "too many open files in system"),
        libc::EMFILE => (0, "too many open files"),
        libc::ENOTTY => (0, "inappropriate ioctl for device"),
        libc::ETXTBSY => (0, "text file busy"),
        libc::EFBIG => (0, "file too large"),
        libc::ENOSPC => (0, "no space left on device"),
        libc::ESPIPE => (0, "illegal seek"),
        libc::EROFS => (0, "read-only file system"),
        libc::EMLINK => (0, "too many links"),
        libc::EPIPE => (0, "broken pipe"),
        libc::EDOM => (0, "numerical argument out of domain"),
        libc::ERANGE => (0, "result too large"),
        libc::EDESTADDRREQ => (0, "destination address required"),
        #[cfg(unix)]
        libc::ELOOP => (0, "too many levels of symbolic links"),
        #[cfg(unix)]
        libc::ENAMETOOLONG => (0, "name too long"),
        libc::EOVERFLOW => (0, "value too large for defined data type"),
        #[cfg(unix)]
        libc::ENOSYS => (0, "function not implemented"),
        libc::EDQUOT => (0, "disk quota exceeded"),
        libc::ENOTEMPTY => (0, "directory not empty"),
        #[cfg(unix)]
        libc::ESTALE => (0, "stale file handle"),
        libc::ENODATA => (0, "no data available"),
        _ => (0, ""),
    };
    match fixed.1 {
        "" => {
            // Fall back to the OS message, lower-casing the first letter like
            // uv does for its own tables.
            let s = io::Error::from_raw_os_error(errno)
                .to_string()
                .replace(&format!(" (os error {})", errno), "");
            let mut c = s.chars();
            match c.next() {
                Some(first) => first.to_lowercase().collect::<String>() + c.as_str(),
                None => s,
            }
        }
        s => s.to_string(),
    }
}

fn errno_code(errno: i32) -> String {
    let code: &'static str = match errno {
        libc::EPERM => "EPERM",
        libc::ENOENT => "ENOENT",
        libc::ESRCH => "ESRCH",
        libc::EINTR => "EINTR",
        libc::EIO => "EIO",
        libc::ENXIO => "ENXIO",
        libc::E2BIG => "E2BIG",
        libc::EBADF => "EBADF",
        libc::EAGAIN => "EAGAIN",
        libc::ENOMEM => "ENOMEM",
        libc::EACCES => "EACCES",
        libc::EFAULT => "EFAULT",
        libc::EBUSY => "EBUSY",
        libc::EEXIST => "EEXIST",
        libc::EXDEV => "EXDEV",
        libc::ENODEV => "ENODEV",
        libc::ENOTDIR => "ENOTDIR",
        libc::EISDIR => "EISDIR",
        libc::EINVAL => "EINVAL",
        libc::ENFILE => "ENFILE",
        libc::EMFILE => "EMFILE",
        libc::ENOTTY => "ENOTTY",
        libc::ETXTBSY => "ETXTBSY",
        libc::EFBIG => "EFBIG",
        libc::ENOSPC => "ENOSPC",
        libc::ESPIPE => "ESPIPE",
        libc::EROFS => "EROFS",
        libc::EMLINK => "EMLINK",
        libc::EPIPE => "EPIPE",
        libc::EDOM => "EDOM",
        libc::ERANGE => "ERANGE",
        #[cfg(unix)]
        libc::ELOOP => "ELOOP",
        #[cfg(unix)]
        libc::ENAMETOOLONG => "ENAMETOOLONG",
        libc::EOVERFLOW => "EOVERFLOW",
        #[cfg(unix)]
        libc::ENOSYS => "ENOSYS",
        libc::EDQUOT => "EDQUOT",
        libc::ENOTEMPTY => "ENOTEMPTY",
        #[cfg(unix)]
        libc::ESTALE => "ESTALE",
        libc::ENODATA => "ENODATA",
        _ => "EUNKNOWN",
    };
    code.to_string()
}

/// A filesystem failure in the exact shape of a Node.js fs error:
/// message `CODE: summary, syscall 'path'` plus code/errno/syscall/path fields,
/// carried as a JSON envelope that the JS glue unpacks into a real JS Error.
#[derive(Debug)]
pub struct FsFail {
    pub code: String,
    pub errno: i32,
    pub syscall: String,
    pub path: String,
    pub summary: String,
}

impl FsFail {
    pub fn new(err: &io::Error, syscall: &str, path: &str) -> Self {
        let errno = err.raw_os_error().unwrap_or(libc::EINVAL);
        FsFail {
            code: errno_code(errno),
            errno: -errno,
            syscall: syscall.to_string(),
            path: path.to_string(),
            summary: errno_summary(errno),
        }
    }

    /// Node-style message for the single-path syscalls
    /// (`ENOENT: no such file or directory, open '/x/y'`).
    pub fn single_message(&self) -> String {
        if self.path.is_empty() {
            format!("{}: {}", self.code, self.summary)
        } else {
            format!("{}: {}, {} '{}'", self.code, self.summary, self.syscall, self.path)
        }
    }

    /// Node-style message for the two-path syscalls (rename, link, symlink,
    /// copyfile): `EEXIST: file already exists, symlink 'dest' -> 'src'`.
    /// `path` holds the first (usually the destination) path.
    pub fn dual_message(&self, second: &str) -> String {
        format!(
            "{}: {}, {} '{}' -> '{}'",
            self.code, self.summary, self.syscall, self.path, second
        )
    }

    pub fn to_js_error(&self, message: String) -> Error {
        Error::new(Status::GenericFailure, envelope_of(self, &message))
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// The error envelope the JS glue unpacks into a real JS Error carrying
/// code/errno/syscall/path like node fs errors do.
pub fn envelope(message: &str) -> String {
    format!("{{\"fsExtraNative\":{{\"message\":\"{}\"}}}}", json_escape(message))
}

pub fn envelope_of(fail: &FsFail, message: &str) -> String {
    format!(
        "{{\"fsExtraNative\":{{\"code\":\"{}\",\"errno\":{},\"syscall\":\"{}\",\"path\":\"{}\",\"message\":\"{}\"}}}}",
        json_escape(&fail.code),
        fail.errno,
        json_escape(&fail.syscall),
        json_escape(&fail.path),
        json_escape(message)
    )
}

/// Convenience: an io::Error converted to a napi Error carrying an fs-style
/// message envelope for the JS glue to unpack.
pub fn fserr(err: &io::Error, syscall: &str, path: &str) -> Error {
    let f = FsFail::new(err, syscall, path);
    let message = f.single_message();
    f.to_js_error(message)
}

pub fn fserr_dual(err: &io::Error, syscall: &str, src: &str, dest: &str) -> Error {
    let f = FsFail::new(err, syscall, src);
    let message = f.dual_message(dest);
    f.to_js_error(message)
}

/// A plain Error (no envelope), like the logical errors fs-extra throws.
/// Sent as an envelope without code fields so the JS glue builds an Error
/// with no code property (napi's Error<Status> would set code GenericFailure).
pub fn plain_error(msg: impl AsRef<str>) -> Error {
    Error::new(
        Status::GenericFailure,
        format!("{{\"fsExtraNative\":{{\"message\":\"{}\"}}}}", json_escape(msg.as_ref())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enoent_envelope() {
        let e = std::fs::symlink_metadata("/nonexistent/rustfs-probe").unwrap_err();
        let f = FsFail::new(&e, "lstat", "/nonexistent/rustfs-probe");
        assert_eq!(f.code, "ENOENT");
        assert_eq!(f.errno, -2);
        let m = f.single_message();
        assert!(m.starts_with("ENOENT: no such file or directory, lstat '/nonexistent/rustfs-probe'"), "{}", m);
        assert!(envelope(&m).starts_with("{\"fsExtraNative\":{\"message\":\"ENOENT"));
    }
}