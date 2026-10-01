use napi_derive::napi;

// Port of node-fs-extra/lib/move/move-sync.js — synchronous move.
use crate::copy_sync::copy_sync_for_move;
use crate::errors::{fserr, fserr_dual, plain_error};
use crate::mkdirs::make_dir_sync_impl;
use crate::paths::{dirname, root_of};
use crate::remove::remove_impl;
use crate::util_stat::{check_paths, check_parent_paths_impl};

#[napi(object)]
#[derive(Debug, Clone, Copy, Default)]
pub struct MoveOpts {
    pub overwrite: Option<bool>,
    pub clobber: Option<bool>,
    pub dereference: Option<bool>,
}

pub fn move_sync_impl(src: &str, dest: &str, opts: Option<MoveOpts>) -> Result<(), napi::Error> {
    let opts = opts.unwrap_or_default();
    let overwrite = opts.overwrite.unwrap_or(false) || opts.clobber.unwrap_or(false);

    let check = check_paths(src, dest, "move", opts.dereference.unwrap_or(false))?;
    check_parent_paths_impl(src, &check.src_stat, dest, "move")?;

    if !is_parent_root(dest) {
        make_dir_sync_impl(&dirname(dest), None)?;
    }
    do_rename(src, dest, overwrite, check.is_changing_case)
}

fn is_parent_root(dest: &str) -> bool {
    root_of(&dirname(dest)) == dirname(dest)
}

fn do_rename(src: &str, dest: &str, overwrite: bool, is_changing_case: bool) -> Result<(), napi::Error> {
    if is_changing_case {
        return rename(src, dest, overwrite);
    }
    if overwrite {
        remove_impl(dest)?;
        return rename(src, dest, overwrite);
    }
    if std::fs::metadata(dest).is_ok() {
        // the source checks fs.existsSync; graceful-fs existsSync
        return Err(plain_error("dest already exists."));
    }
    rename(src, dest, overwrite)
}

pub(crate) fn rename(src: &str, dest: &str, overwrite: bool) -> Result<(), napi::Error> {
    match rename_raw(src, dest) {
        Err(err) => {
            // Try rename first, and try copy + remove if EXDEV
            if err.to_string().contains("EXDEV") {
                move_across_device_sync(src, dest, overwrite)
            } else {
                Err(err)
            }
        }
        Ok(()) => Ok(()),
    }
}

pub(crate) fn rename_raw(src: &str, dest: &str) -> Result<(), napi::Error> {
    let csrc = std::ffi::CString::new(src.as_bytes().to_vec()).map_err(|_| {
        fserr(&std::io::Error::from_raw_os_error(libc::EINVAL), "rename", src)
    });
    let cdst = std::ffi::CString::new(dest.as_bytes().to_vec()).map_err(|_| {
        fserr(&std::io::Error::from_raw_os_error(libc::EINVAL), "rename", dest)
    });
    let (csrc, cdst) = (match csrc { Ok(c) => c, Err(e) => return Err(e) }, match cdst { Ok(c) => c, Err(e) => return Err(e) });
    let rc = unsafe { libc::rename(csrc.as_ptr(), cdst.as_ptr()) };
    if rc == 0 {
        return Ok(());
    }
    let err = std::io::Error::last_os_error();
    Err(fserr_dual(&err, "rename", src, dest))
}

fn move_across_device_sync(src: &str, dest: &str, overwrite: bool) -> Result<(), napi::Error> {
    let opts = crate::copy_sync::CopyOpts {
        overwrite,
        error_on_exist: true,
        preserve_timestamps: true,
        dereference: false, // the source passes its own options here; moveAcrossDevice has none
    };
    copy_sync_for_move(src, dest, opts)?;
    remove_impl(src)
}