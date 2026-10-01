// Port of node-fs-extra/lib/output-file/index.js: writeFile+mkdirs in one
// native step (the source does existsSync + mkdirs + writeFile).
use crate::ensure::{write_file_sync_impl};
use crate::mkdirs::make_dir_impl;
use crate::util_utimes::Times;

/// outputFileSync core: ensure the parent dir exists, then write.
pub fn output_file_sync_impl(
    file: &str,
    data: &[u8],
    mode: u32,
    flag: &str,
) -> Result<(), napi::Error> {
    let dir = crate::paths::dirname(file);
    if std::fs::metadata(&dir).is_err() {
        make_dir_impl(&dir, None)?;
    }
    write_file_sync_impl(file, data, mode, flag)
}

/// outputFile async core (same work on the libuv thread).
pub fn output_file_impl(file: &str, data: Vec<u8>, mode: u32, flag: &str) -> Result<(), napi::Error> {
    let _ = Times::default();
    output_file_sync_impl(file, &data, mode, flag)
}