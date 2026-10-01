use napi_derive::napi;

// Port of node-fs-extra/lib/copy/copy-sync.js — synchronous copy.
use std::io;

use rayon::prelude::*;

use crate::ensure::symlink_sync;
use crate::errors::{fserr, fserr_dual, plain_error};
use crate::mkdirs::make_dir_sync_impl;
use crate::paths::{dirname, is_src_subdir, resolve};
use crate::util_stat::{check_paths, check_parent_paths_impl, lstat, stat, St};

#[napi(object)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CopyOpts {
    pub overwrite: bool,
    pub error_on_exist: bool,
    pub preserve_timestamps: bool,
    pub dereference: bool,
}

/// Options normalization shared by copy and copySync (the JS glue handles the
/// `filter as function` shorthand; `overwrite` falls back to `clobber`).
pub fn opts_from(
    clobber: Option<bool>,
    overwrite: Option<bool>,
    error_on_exist: Option<bool>,
    preserve_timestamps: Option<bool>,
    dereference: Option<bool>,
) -> CopyOpts {
    let clobber = clobber.unwrap_or(true); // default to true for now
    CopyOpts {
        overwrite: overwrite.unwrap_or(clobber), // overwrite falls back to clobber
        error_on_exist: error_on_exist.unwrap_or(false),
        preserve_timestamps: preserve_timestamps.unwrap_or(false),
        dereference: dereference.unwrap_or(false),
    }
}

pub fn copy_sync_dispatch(
    src: &str,
    dest: &str,
    opts: CopyOpts,
    filter: Option<(napi::Env, napi::JsFunction)>,
) -> Result<(), napi::Error> {
    // Warn about using preserveTimestamps on 32-bit node: only the ia32
    // platform gets the fs-extra-WARN0002 warning in the source; on this
    // build (64-bit) the condition is never true, so no warning is emitted.

    let check = check_paths(src, dest, "copy", opts.dereference)?;
    check_parent_paths_impl(src, &check.src_stat, dest, "copy")?;

    let filter: Option<&(napi::Env, napi::JsFunction)> = filter.as_ref();
    if let Some((env, filter_fn)) = filter {
        if !run_filter_sync(env, filter_fn, src, dest) {
            return Ok(());
        }
    }

    let dest_parent = dirname(dest);
    if std::fs::metadata(&dest_parent).is_err() {
        make_dir_sync_impl(&dest_parent, None)?;
    }
    get_stats(check.dest_stat, src, dest, &opts, filter)
}

/// move's cross-device fallback calls the sync copy with its own options and
/// no filter.
pub fn copy_sync_for_move(src: &str, dest: &str, opts: CopyOpts) -> Result<(), napi::Error> {
    copy_sync_dispatch(src, dest, opts, None)
}

/// runFilter in the source: `opts.filter(src, dest)` with unawaited truthiness
/// for the sync path (a returned promise is an object, hence truthy).
fn run_filter_sync(env: &napi::Env, filter: &napi::JsFunction, src: &str, dest: &str) -> bool {
    let src_js = match env.create_string(src) {
        Ok(s) => s,
        Err(_) => return true,
    };
    let dest_js = match env.create_string(dest) {
        Ok(s) => s,
        Err(_) => return true,
    };
    match filter.call(None, &[&src_js, &dest_js]) {
        // a throwing filter surfaces as a pending JS exception and interrupts
        // sync execution, like the source
        Ok(v) => truthy(env, &v),
        Err(_) => true,
    }
}

/// JS truthiness of an arbitrary value.
pub fn truthy(env: &napi::Env, v: &napi::JsUnknown) -> bool {
    use napi::JsBoolean;
    use napi::JsNumber;
    use napi::NapiRaw;
    use napi::NapiValue;
    use napi::ValueType;
    match v.get_type() {
        Ok(t) => match t {
            ValueType::Undefined | ValueType::Null => false,
            ValueType::Boolean => unsafe {
                JsBoolean::from_raw_unchecked(env.raw(), v.raw())
                    .get_value()
                    .unwrap_or(false)
            },
            ValueType::Number => unsafe {
                let n = JsNumber::from_raw_unchecked(env.raw(), v.raw())
                    .get_double()
                    .unwrap_or(f64::NAN);
                !n.is_nan() && n != 0.0
            },
            ValueType::String => unsafe {
                let mut len = 0usize;
                let rc = napi::sys::napi_get_value_string_utf8(env.raw(), v.raw(), std::ptr::null_mut(), 0, &mut len);
                if rc != 0 {
                    true
                } else {
                    len == 0
                }
            },
            _ => true,
        },
        Err(_) => true,
    }
}

fn get_stats(
    dest_stat: Option<St>,
    src: &str,
    dest: &str,
    opts: &CopyOpts,
    filter: Option<&(napi::Env, napi::JsFunction)>,
) -> Result<(), napi::Error> {
    let src_stat = if opts.dereference {
        stat(src).map_err(|e| fserr(&e, "stat", src))?
    } else {
        lstat(src).map_err(|e| fserr(&e, "lstat", src))?
    };

    if src_stat.is_dir() {
        on_dir(src_stat, dest_stat, src, dest, opts, filter)
    } else if src_stat.is_file() || src_stat.is_char_device() || src_stat.is_block_device() {
        on_file(src_stat, dest_stat, src, dest, opts)
    } else if src_stat.is_symlink() {
        on_link(dest_stat, src, dest, opts)
    } else if src_stat.is_socket() {
        Err(plain_error(format!("Cannot copy a socket file: {}", src)))
    } else if src_stat.is_fifo() {
        Err(plain_error(format!("Cannot copy a FIFO pipe: {}", src)))
    } else {
        Err(plain_error(format!("Unknown file: {}", src)))
    }
}

fn on_file(
    src_stat: St,
    dest_stat: Option<St>,
    src: &str,
    dest: &str,
    opts: &CopyOpts,
) -> Result<(), napi::Error> {
    if dest_stat.is_none() {
        return copy_file(src_stat, src, dest, opts);
    }
    may_copy_file(src_stat, src, dest, opts)
}

fn may_copy_file(
    src_stat: St,
    src: &str,
    dest: &str,
    opts: &CopyOpts,
) -> Result<(), napi::Error> {
    if opts.overwrite {
        unlink_sync(dest)?;
        return copy_file(src_stat, src, dest, opts);
    }
    if opts.error_on_exist {
        return Err(plain_error(format!("'{}' already exists", dest)));
    }
    Ok(())
}

pub(crate) fn unlink_sync(path: &str) -> Result<(), napi::Error> {
    let c = match std::ffi::CString::new(path.as_bytes().to_vec()) {
        Ok(c) => c,
        Err(_) => {
            return Err(fserr(&io::Error::from_raw_os_error(libc::EINVAL), "unlink", path))
        }
    };
    let rc = unsafe { libc::unlink(c.as_ptr()) };
    if rc == 0 {
        Ok(())
    } else {
        let e = io::Error::last_os_error();
        Err(fserr(&e, "unlink", path))
    }
}

pub(crate) fn chmod_sync(path: &str, mode: u32) -> Result<(), napi::Error> {
    let c = match std::ffi::CString::new(path.as_bytes().to_vec()) {
        Ok(c) => c,
        Err(_) => {
            return Err(fserr(&io::Error::from_raw_os_error(libc::EINVAL), "chmod", path))
        }
    };
    let rc = unsafe { libc::chmod(c.as_ptr(), mode as libc::mode_t) };
    if rc == 0 {
        Ok(())
    } else {
        let e = io::Error::last_os_error();
        Err(fserr(&e, "chmod", path))
    }
}

pub(crate) fn copy_file(
    src_stat: St,
    src: &str,
    dest: &str,
    opts: &CopyOpts,
) -> Result<(), napi::Error> {
    // fs.copyFile follows symlinks; regular files go through std::fs::copy
    // (fclonefileat/fcopyfile on darwin, preserving the mode exactly, like
    // node). Non-regular files (device nodes etc.) fall back to a plain
    // open(0o666&umask)+read/write loop, matching node's behavior.
    let follows = std::fs::metadata(src).map_err(|e| fserr_dual(&e, "copyfile", src, dest))?;
    if follows.file_type().is_file() {
        std::fs::copy(src, dest).map_err(|e| fserr_dual(&e, "copyfile", src, dest))?;
    } else {
        copy_via_rw(src, dest).map_err(|e| fserr_dual(&e, "copyfile", src, dest))?;
    }
    if opts.preserve_timestamps {
        handle_timestamps(src_stat, src, dest)?;
    }
    chmod_sync(dest, src_stat.mode & 0o7777)
}

fn copy_via_rw(src: &str, dest: &str) -> io::Result<()> {
    use std::io::{Read, Write};
    let mut reader = std::fs::File::open(src)?;
    let mut writer = std::fs::OpenOptions::new()
        .write(true)
        .create_new(false)
        .create(true)
        .truncate(true)
        .open(dest)?;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n])?;
    }
    Ok(())
}

fn file_is_not_writable(src_mode: u32) -> bool {
    (src_mode & 0o200) == 0
}

fn handle_timestamps(src_stat: St, src: &str, dest: &str) -> Result<(), napi::Error> {
    // Make sure the file is writable before setting the timestamp, otherwise
    // open fails with EPERM when invoked with 'r+' (through utimes call)
    if file_is_not_writable(src_stat.mode) {
        chmod_sync(dest, (src_stat.mode | 0o200) & 0o7777)?;
    }
    set_dest_timestamps(src, dest)
}

fn set_dest_timestamps(src: &str, dest: &str) -> Result<(), napi::Error> {
    // The initial srcStat.atime cannot be trusted because it is modified by
    // the read(2) system call, so re-stat the source (fs.stat follows links).
    let updated_src_stat = stat(src).map_err(|e| fserr(&e, "stat", src))?;
    crate::util_utimes::utimes_millis_sync_impl(
        dest,
        updated_src_stat.times.atime,
        updated_src_stat.times.mtime,
    )
}

fn on_dir(
    src_stat: St,
    dest_stat: Option<St>,
    src: &str,
    dest: &str,
    opts: &CopyOpts,
    filter: Option<&(napi::Env, napi::JsFunction)>,
) -> Result<(), napi::Error> {
    if dest_stat.is_none() {
        return mk_dir_and_copy(src_stat, src, dest, opts, filter);
    }
    copy_dir(src, dest, opts, filter)
}

fn mk_dir_and_copy(
    src_mode: St,
    src: &str,
    dest: &str,
    opts: &CopyOpts,
    filter: Option<&(napi::Env, napi::JsFunction)>,
) -> Result<(), napi::Error> {
    mkdir_bare(dest)?;
    copy_dir(src, dest, opts, filter)?;
    chmod_sync(dest, src_mode.mode & 0o7777)
}

fn mkdir_bare(dest: &str) -> Result<(), napi::Error> {
    let c = match std::ffi::CString::new(dest.as_bytes().to_vec()) {
        Ok(c) => c,
        Err(_) => {
            return Err(fserr(&io::Error::from_raw_os_error(libc::EINVAL), "mkdir", dest))
        }
    };
    let rc = unsafe { libc::mkdir(c.as_ptr(), 0o777 as libc::mode_t) };
    if rc == 0 {
        Ok(())
    } else {
        let e = io::Error::last_os_error();
        Err(fserr(&e, "mkdir", dest))
    }
}

fn copy_dir(
    src: &str,
    dest: &str,
    opts: &CopyOpts,
    filter: Option<&(napi::Env, napi::JsFunction)>,
) -> Result<(), napi::Error> {
    let raw_entries = std::fs::read_dir(src).map_err(|e| fserr(&e, "opendir", src))?;
    let mut names: Vec<String> = Vec::new();
    for entry in raw_entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => return Err(fserr(&e, "readdir", src)),
        };
        names.push(entry.file_name().to_string_lossy().into_owned());
    }

    if filter.is_none() {
        // no filter: copy the items in parallel (order-insensitive; the first
        // failure in directory order wins, like the sequential source)
        let errors: Vec<(usize, napi::Error)> = names
            .into_par_iter()
            .enumerate()
            .filter_map(|(index, name)| {
                copy_dir_item(&name, src, dest, opts, None).err().map(|e| (index, e))
            })
            .collect();
        if let Some((_, first)) = errors.into_iter().min_by_key(|(i, _)| *i) {
            return Err(first);
        }
        return Ok(());
    }

    for name in names {
        copy_dir_item(&name, src, dest, opts, filter)?;
    }
    Ok(())
}

fn copy_dir_item(
    item: &str,
    src: &str,
    dest: &str,
    opts: &CopyOpts,
    filter: Option<&(napi::Env, napi::JsFunction)>,
) -> Result<(), napi::Error> {
    let (src_item, dest_item) = join_pair(src, dest, item);
    if let Some((env, filter_fn)) = filter {
        if !run_filter_sync(&env, &filter_fn, &src_item, &dest_item) {
            return Ok(());
        }
    }
    let check = check_paths(&src_item, &dest_item, "copy", opts.dereference)?;
    get_stats(check.dest_stat, &src_item, &dest_item, opts, filter)
}

pub(crate) fn join_pair(src: &str, dest: &str, item: &str) -> (String, String) {
    let src_item = if src.ends_with('/') {
        format!("{}{}", src, item)
    } else {
        format!("{}/{}", src, item)
    };
    let dest_item = if dest.ends_with('/') {
        format!("{}{}", dest, item)
    } else {
        format!("{}/{}", dest, item)
    };
    (src_item, dest_item)
}

fn on_link(
    dest_stat: Option<St>,
    src: &str,
    dest: &str,
    opts: &CopyOpts,
) -> Result<(), napi::Error> {
    let mut resolved_src = readlink_sync(src)?;
    if opts.dereference {
        resolved_src = resolve(&resolved_src);
    }

    if dest_stat.is_none() {
        return symlink_sync(&resolved_src, dest, "file");
    }

    let mut resolved_dest;
    match readlink_sync(dest) {
        Ok(r) => resolved_dest = r,
        Err(err) => {
            // dest exists and is a regular file or directory. Windows may
            // throw UNKNOWN; if dest already exists, fs throws error anyway,
            // so only EINVAL/UNKNOWN become a fresh symlink attempt here.
            let msg = err.to_string();
            if msg.contains("EINVAL") || msg.contains("UNKNOWN") {
                return symlink_sync(&resolved_src, dest, "file");
            }
            return Err(err);
        }
    }
    if opts.dereference {
        resolved_dest = resolve(&resolved_dest);
    }
    // If both symlinks resolve to the same target, they are still distinct
    // symlinks that can be copied/overwritten. Only check subdirectory
    // constraints when the resolved paths are different.
    if resolved_src != resolved_dest {
        if is_src_subdir(&resolved_src, &resolved_dest) {
            return Err(plain_error(format!(
                "Cannot copy '{}' to a subdirectory of itself, '{}'.",
                resolved_src, resolved_dest
            )));
        }

        // prevent copy if src is a subdir of dest since unlinking dest in this
        // case would result in removing src contents and therefore a broken
        // symlink would be created.
        if is_src_subdir(&resolved_dest, &resolved_src) {
            return Err(plain_error(format!(
                "Cannot overwrite '{}' with '{}'.",
                resolved_dest, resolved_src
            )));
        }
    }
    // copy the link
    unlink_sync(dest)?;
    symlink_sync(&resolved_src, dest, "file")
}

pub(crate) fn readlink_sync(path: &str) -> Result<String, napi::Error> {
    let c = match std::ffi::CString::new(path.as_bytes().to_vec()) {
        Ok(c) => c,
        Err(_) => {
            return Err(fserr(&io::Error::from_raw_os_error(libc::EINVAL), "readlink", path))
        }
    };
    let mut buf = vec![0u8; 256];
    loop {
        let len = unsafe { libc::readlink(c.as_ptr(), buf.as_mut_ptr().cast(), buf.len()) };
        if len < 0 {
            let e = io::Error::last_os_error();
            return Err(fserr(&e, "readlink", path));
        }
        let len = len as usize;
        if len < buf.len() {
            buf.truncate(len);
            return match String::from_utf8(buf) {
                Ok(s) => Ok(s),
                Err(_) => {
                    Err(fserr(&io::Error::from_raw_os_error(libc::EINVAL), "readlink", path))
                }
            };
        }
        buf.resize(buf.len() * 2, 0);
    }
}