// Port of node-fs-extra/lib/empty/index.js: empty a directory, or create it
// when it does not exist.
use rayon::prelude::*;

use crate::errors::fserr;
use crate::mkdirs::make_dir_impl;
use crate::remove::remove_impl;

pub fn empty_dir_sync_impl(dir: &str) -> Result<(), napi::Error> {
    let items = match std::fs::read_dir(dir) {
        Ok(items) => items,
        Err(e) => {
            if e.raw_os_error() == Some(libc::ENOENT) {
                // The directory does not exist, create it (mkdirs)
                return make_dir_impl(dir, None);
            }
            return Err(fserr(&e, "scandir", dir));
        }
    };
    let children: Vec<String> = items
        .filter_map(|e| e.ok())
        .map(|e| {
            let name = e.file_name();
            format!("{}/{}", dir.trim_end_matches('/'), name.to_string_lossy())
        })
        .collect();
    let errors: Vec<napi::Error> = children
        .into_par_iter()
        .filter_map(|child| remove_impl(&child).err())
        .collect();
    if let Some(first) = errors.into_iter().next() {
        return Err(first);
    }
    Ok(())
}