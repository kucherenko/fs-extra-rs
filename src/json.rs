// Port of node-fs-extra/lib/json/output-json{,-sync}.js + readJson/writeJson
// helpers. JSON.stringify / JSON.parse happen in the JS glue (exact engine
// parity); this core does the file IO: read the file (utf8), or ensure the
// parent and write it.
use crate::output_file::output_file_sync_impl;

/// readJson core: read the file as utf8 text (identical to
/// JSON.parse(await readFile(file)) which the JS layer parses).
pub fn read_json_sync_impl(file: &str) -> Result<String, napi::Error> {
    let bytes = std::fs::read(file).map_err(|e| {
        crate::errors::fserr(&e, "open", file)
    })?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// writeJson core: write serialized JSON (flag 'w', mode 0o666).
pub fn write_json_sync_impl(file: &str, data: &[u8], mode: u32, flag: &str) -> Result<(), napi::Error> {
    crate::ensure::write_file_sync_impl(file, data, mode, flag)
}

/// outputJson core: ensure parent dirs, then write serialized JSON.
pub fn output_json_sync_impl(
    file: &str,
    data: &[u8],
    mode: u32,
) -> Result<(), napi::Error> {
    output_file_sync_impl(file, data, mode, "w")
}