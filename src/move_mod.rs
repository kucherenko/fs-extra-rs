// Port of node-fs-extra/lib/move/move.js — asynchronous move.
use crate::copy::{async_copy, Filter};
use crate::copy_sync::CopyOpts;
use crate::errors::plain_error;
use crate::move_sync::{rename_raw, MoveOpts};
use crate::mkdirs::make_dir_impl;
use crate::paths::{dirname, root_of};
use crate::remove::remove_impl;
use crate::util_stat::{check_paths, check_parent_paths_impl};

/// Port of the async `move(src, dest, opts)` function; the overwrite/clobber
/// normalization reads `opts.overwrite || opts.clobber || false`.
pub fn move_dispatch(src: String, dest: String, opts: Option<MoveOpts>) -> Result<(), napi::Error> {
    let opts = opts.unwrap_or_default();
    let overwrite = opts.overwrite.unwrap_or(false) || opts.clobber.unwrap_or(false);

    let check = check_paths(&src, &dest, "move", opts.dereference.unwrap_or(false))?;
    check_parent_paths_impl(&src, &check.src_stat, &dest, "move")?;

    // If the parent of dest is not root, make sure it exists before proceeding
    let dest_parent = dirname(&dest);
    if root_of(&dest_parent) != dest_parent {
        make_dir_impl(&dest_parent, None)?;
    }

    do_rename(&src, &dest, overwrite, check.is_changing_case)
}

fn do_rename(src: &str, dest: &str, overwrite: bool, is_changing_case: bool) -> Result<(), napi::Error> {
    if !is_changing_case {
        if overwrite {
            remove_impl(dest)?;
        } else if std::fs::metadata(dest).is_ok() {
            // the source checks pathExists(dest) (fs.access), which resolves
            // false exactly when the path cannot be stat'ed
            return Err(plain_error("dest already exists."));
        }
    }

    // Try rename first, and try copy + remove if EXDEV
    match rename_raw(src, dest) {
        Ok(()) => Ok(()),
        Err(err) => {
            if is_exdev(&err) {
                move_across_device(src, dest, overwrite)
            } else {
                Err(err)
            }
        }
    }
}

pub(crate) fn is_exdev(err: &napi::Error) -> bool {
    err.to_string().contains("EXDEV")
}

fn move_across_device(src: &str, dest: &str, overwrite: bool) -> Result<(), napi::Error> {
    let opts = CopyOpts {
        overwrite,
        error_on_exist: true,
        preserve_timestamps: true,
        dereference: false,
    };
    async_copy(src.to_string(), dest.to_string(), opts, Filter::None)?;
    remove_impl(src)
}