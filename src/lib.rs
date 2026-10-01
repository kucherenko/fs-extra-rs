#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]
//! Native core of @jscpd/fs-extra: the fs-extra API implemented in Rust and
//! exposed as a Node.js addon through napi-rs. The JS layers in lib/ glue
//! these exports into the exact fs-extra API (universalify, aliases,
//! graceful-fs passthrough).

use napi_derive::napi;

mod copy;
mod copy_sync;
mod empty;
mod ensure;
mod errors;
mod json;
mod mkdirs;
mod move_mod;
mod move_sync;
mod napi_internal;
mod output_file;
mod paths;
mod remove;
mod util_stat;
mod util_utimes;

use napi::bindgen_prelude::AsyncTask;
use napi::bindgen_prelude::Buffer;
use napi::{Either};
use std::sync::Arc;
use napi::{Env, JsFunction, Result, Task};

use crate::copy::{Filter, FilterTsfn};

// ---------------------------------------------------------------------------
// Shared argument types
// ---------------------------------------------------------------------------

#[napi(object)]
#[derive(Debug, Clone, Copy, Default)]
pub struct JsCopyOptions {
    pub clobber: Option<bool>,
    pub overwrite: Option<bool>,
    pub error_on_exist: Option<bool>,
    pub preserve_timestamps: Option<bool>,
    pub dereference: Option<bool>,
}

#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct JsWriteOptions {
    pub mode: Option<u32>,
    pub flag: Option<String>,
}

// ---------------------------------------------------------------------------
// Task plumbing: every async variant runs its (identical) blocking core on
// the libuv threadpool through napi's AsyncTask.
// ---------------------------------------------------------------------------

macro_rules! impl_task {
    ($name:ident, $out:ty) => {
        impl Task for $name {
            type Output = $out;
            type JsValue = $out;
            fn compute(&mut self) -> Result<$out> {
                self.compute_work()
            }
            fn resolve(&mut self, _env: Env, output: $out) -> Result<$out> {
                Ok(output)
            }
        }
    };
}

// ---------------------------------------------------------------------------
// copy / copySync
// ---------------------------------------------------------------------------

pub struct CopyTask {
    pub src: String,
    pub dest: String,
    pub opts: copy_sync::CopyOpts,
    pub filter: Filter,
}

impl CopyTask {
    fn compute_work(&mut self) -> Result<()> {
        copy::copy_dispatch(self.src.clone(), self.dest.clone(), self.opts, self.filter.clone())
    }
}

impl_task!(CopyTask, ());

#[napi(js_name = "copySync")]
pub fn copy_sync(
    env: Env,
    src: String,
    dest: String,
    options: Option<JsCopyOptions>,
    filter: Option<JsFunction>,
) -> Result<()> {
    let o = options.unwrap_or_default();
    let opts = copy_sync::opts_from(
        o.clobber,
        o.overwrite,
        o.error_on_exist,
        o.preserve_timestamps,
        o.dereference,
    );
    copy_sync::copy_sync_dispatch(&src, &dest, opts, filter.map(|f| (env, f)))
}

#[napi(js_name = "copyAsync")]
pub fn copy_async(
    env: Env,
    src: String,
    dest: String,
    options: Option<JsCopyOptions>,
    filter: Option<JsFunction>,
) -> Result<AsyncTask<CopyTask>> {
    let o = options.unwrap_or_default();
    let opts = copy_sync::opts_from(
        o.clobber,
        o.overwrite,
        o.error_on_exist,
        o.preserve_timestamps,
        o.dereference,
    );
    // The JS glue passes an adapter invoked as (err, src, dest) whose return
    // is a promise of a boolean include answer.
    let filter = match filter {
        Some(f) => {
            let tsfn: FilterTsfn = env.create_threadsafe_function::<(String, String), String, _>(
                &f, 0, |ctx| Ok(vec![ctx.value.0.clone(), ctx.value.1.clone()]),
            )?;
            Filter::Tsfn(Arc::new(tsfn))
        }
        None => Filter::None,
    };
    Ok(AsyncTask::new(CopyTask { src, dest, opts, filter }))
}

// ---------------------------------------------------------------------------
// move / moveSync
// ---------------------------------------------------------------------------

pub struct MoveTask {
    pub src: String,
    pub dest: String,
    pub opts: Option<move_sync::MoveOpts>,
}

impl MoveTask {
    fn compute_work(&mut self) -> Result<()> {
        move_mod::move_dispatch(self.src.clone(), self.dest.clone(), self.opts)
    }
}

impl_task!(MoveTask, ());

#[napi(js_name = "moveSync")]
pub fn move_sync(src: String, dest: String, options: Option<move_sync::MoveOpts>) -> Result<()> {
    move_sync::move_sync_impl(&src, &dest, options)
}

#[napi(js_name = "moveAsync")]
pub fn move_async(
    src: String,
    dest: String,
    options: Option<move_sync::MoveOpts>,
) -> Result<AsyncTask<MoveTask>> {
    Ok(AsyncTask::new(MoveTask { src, dest, opts: options }))
}

// ---------------------------------------------------------------------------
// mkdirs / mkdirsSync (aliases mkdirp/ensureDir via the JS glue)
// ---------------------------------------------------------------------------

pub struct MkdirsTask {
    pub dir: String,
    pub mode: Option<i32>,
}

impl MkdirsTask {
    fn compute_work(&mut self) -> Result<Option<String>> {
        mkdirs::make_dir_sync_impl(&self.dir, self.mode)
    }
}

impl_task!(MkdirsTask, Option<String>);

#[napi(js_name = "mkdirsSync")]
pub fn mkdirs_sync(dir: String, options: Option<Either<u32, MkdirOptions>>) -> Result<Option<String>> {
    mkdirs::make_dir_sync_impl(&dir, mkdir_mode(options))
}

#[napi(js_name = "mkdirsAsync")]
pub fn mkdirs_async(
    dir: String,
    options: Option<Either<u32, MkdirOptions>>,
) -> Result<AsyncTask<MkdirsTask>> {
    Ok(AsyncTask::new(MkdirsTask { dir, mode: mkdir_mode(options) }))
}

#[napi(object)]
#[derive(Debug, Clone, Copy, Default)]
pub struct MkdirOptions {
    pub mode: Option<u32>,
}

fn mkdir_mode(options: Option<Either<u32, MkdirOptions>>) -> Option<i32> {
    match options {
        None => None,
        Some(Either::A(num)) => Some(num as i32),
        Some(Either::B(obj)) => obj.mode.map(|m| m as i32),
    }
}

// ---------------------------------------------------------------------------
// remove / removeSync
// ---------------------------------------------------------------------------

pub struct RemoveTask {
    pub path: String,
}

impl RemoveTask {
    fn compute_work(&mut self) -> Result<()> {
        remove::remove_impl(&self.path)
    }
}

impl_task!(RemoveTask, ());

#[napi(js_name = "removeSync")]
pub fn remove_sync(path: String) -> Result<()> {
    remove::remove_impl(&path)
}

#[napi(js_name = "removeAsync")]
pub fn remove_async(path: String) -> Result<AsyncTask<RemoveTask>> {
    Ok(AsyncTask::new(RemoveTask { path }))
}

// ---------------------------------------------------------------------------
// emptyDir / emptyDirSync (aliases emptydir* via the JS glue)
// ---------------------------------------------------------------------------

pub struct EmptyDirTask {
    pub dir: String,
}

impl EmptyDirTask {
    fn compute_work(&mut self) -> Result<()> {
        empty::empty_dir_sync_impl(&self.dir)
    }
}

impl_task!(EmptyDirTask, ());

#[napi(js_name = "emptyDirSync")]
pub fn empty_dir_sync(dir: String) -> Result<()> {
    empty::empty_dir_sync_impl(&dir)
}

#[napi(js_name = "emptyDirAsync")]
pub fn empty_dir_async(dir: String) -> Result<AsyncTask<EmptyDirTask>> {
    Ok(AsyncTask::new(EmptyDirTask { dir }))
}

// ---------------------------------------------------------------------------
// createFile / createFileSync (ensureFile via glue)
// ---------------------------------------------------------------------------

pub struct CreateFileTask {
    pub file: String,
}

impl CreateFileTask {
    fn compute_work(&mut self) -> Result<()> {
        ensure::create_file_sync_impl(&self.file)
    }
}

impl_task!(CreateFileTask, ());

#[napi(js_name = "createFileSync")]
pub fn create_file_sync(file: String) -> Result<()> {
    ensure::create_file_sync_impl(&file)
}

#[napi(js_name = "createFileAsync")]
pub fn create_file_async(file: String) -> Result<AsyncTask<CreateFileTask>> {
    Ok(AsyncTask::new(CreateFileTask { file }))
}

// ---------------------------------------------------------------------------
// createLink / createLinkSync (ensureLink via glue)
// ---------------------------------------------------------------------------

pub struct CreateLinkTask {
    pub src: String,
    pub dst: String,
}

impl CreateLinkTask {
    fn compute_work(&mut self) -> Result<()> {
        ensure::create_link_sync_impl(&self.src, &self.dst)
    }
}

impl_task!(CreateLinkTask, ());

#[napi(js_name = "createLinkSync")]
pub fn create_link_sync(src: String, dst: String) -> Result<()> {
    ensure::create_link_sync_impl(&src, &dst)
}

#[napi(js_name = "createLinkAsync")]
pub fn create_link_async(src: String, dst: String) -> Result<AsyncTask<CreateLinkTask>> {
    Ok(AsyncTask::new(CreateLinkTask { src, dst }))
}

// ---------------------------------------------------------------------------
// createSymlink / createSymlinkSync (ensureSymlink via glue)
// ---------------------------------------------------------------------------

pub struct CreateSymlinkTask {
    pub src: String,
    pub dst: String,
    pub symlink_type: Option<String>,
}

impl CreateSymlinkTask {
    fn compute_work(&mut self) -> Result<()> {
        ensure::create_symlink_sync_impl(&self.src, &self.dst, self.symlink_type.as_deref())
    }
}

impl_task!(CreateSymlinkTask, ());

#[napi(js_name = "createSymlinkSync")]
pub fn create_symlink_sync(src: String, dst: String, _type: Option<String>) -> Result<()> {
    ensure::create_symlink_sync_impl(&src, &dst, _type.as_deref())
}

#[napi(js_name = "createSymlinkAsync")]
pub fn create_symlink_async(
    src: String,
    dst: String,
    _type: Option<String>,
) -> Result<AsyncTask<CreateSymlinkTask>> {
    Ok(AsyncTask::new(CreateSymlinkTask { src, dst, symlink_type: _type }))
}

// ---------------------------------------------------------------------------
// outputFile / outputFileSync
// ---------------------------------------------------------------------------

pub struct OutputFileTask {
    pub file: String,
    pub data: Vec<u8>,
    pub mode: u32,
    pub flag: String,
}

impl OutputFileTask {
    fn compute_work(&mut self) -> Result<()> {
        output_file::output_file_impl(&self.file, self.data.clone(), self.mode, &self.flag)
    }
}

impl_task!(OutputFileTask, ());

#[napi(js_name = "outputFileSync")]
pub fn output_file_sync(
    file: String,
    data: Either<String, Buffer>,
    options: Option<Either<String, JsWriteOptions>>,
) -> Result<()> {
    let (data, mode, flag) = normalize_write_args(data, options)?;
    output_file::output_file_sync_impl(&file, &data, mode, &flag)
}

#[napi(js_name = "outputFileAsync")]
pub fn output_file_async(
    file: String,
    data: Either<String, Buffer>,
    options: Option<Either<String, JsWriteOptions>>,
) -> Result<AsyncTask<OutputFileTask>> {
    let (data, mode, flag) = normalize_write_args(data, options)?;
    Ok(AsyncTask::new(OutputFileTask { file, data, mode, flag }))
}

/// Normalizes (data, writeFile-options) the way node fs.write does: the
/// default encoding comes from the options (utf-8 handled natively; the JS
/// glue routes other encodings to the graceful-fs fallback), default
/// mode 0o666, default flag w.
fn normalize_write_args(
    data: Either<String, Buffer>,
    options: Option<Either<String, JsWriteOptions>>,
) -> Result<(Vec<u8>, u32, String)> {
    let bytes = match data {
        Either::A(s) => s.into_bytes(),
        Either::B(b) => (&*b).to_vec(),
    };
    let mut mode = 0o666u32;
    let mut flag = String::from("w");
    if let Some(Either::B(obj)) = options {
        if let Some(m) = obj.mode {
            mode = m;
        }
        if let Some(f) = &obj.flag {
            flag = f.clone();
        }
    }
    Ok((bytes, mode, flag))
}

// ---------------------------------------------------------------------------
// JSON cores (stringify/parse happen in the JS glue)
// ---------------------------------------------------------------------------

pub struct ReadJsonTask {
    pub file: String,
}

impl ReadJsonTask {
    fn compute_work(&mut self) -> Result<String> {
        json::read_json_sync_impl(&self.file)
    }
}

impl_task!(ReadJsonTask, String);

pub struct WriteJsonTask {
    pub file: String,
    pub data: String,
    pub mode: u32,
    pub flag: String,
}

impl WriteJsonTask {
    fn compute_work(&mut self) -> Result<()> {
        json::write_json_sync_impl(&self.file, self.data.as_bytes(), self.mode, &self.flag)
    }
}

impl_task!(WriteJsonTask, ());

pub struct OutputJsonTask {
    pub file: String,
    pub data: String,
    pub mode: u32,
}

impl OutputJsonTask {
    fn compute_work(&mut self) -> Result<()> {
        json::output_json_sync_impl(&self.file, self.data.as_bytes(), self.mode)
    }
}

impl_task!(OutputJsonTask, ());

#[napi(js_name = "outputJsonSync")]
pub fn output_json_sync(file: String, data: String, options: Option<JsWriteOptions>) -> Result<()> {
    let (mode, _flag) = write_options(options);
    json::output_json_sync_impl(&file, data.as_bytes(), mode)
}

#[napi(js_name = "outputJsonAsync")]
pub fn output_json_async(
    file: String,
    data: String,
    options: Option<JsWriteOptions>,
) -> Result<AsyncTask<OutputJsonTask>> {
    let (mode, _flag) = write_options(options);
    Ok(AsyncTask::new(OutputJsonTask { file, data, mode }))
}

#[napi(js_name = "readJsonSync")]
pub fn read_json_sync(file: String) -> Result<String> {
    json::read_json_sync_impl(&file)
}

#[napi(js_name = "readJsonAsync")]
pub fn read_json_async(file: String) -> Result<AsyncTask<ReadJsonTask>> {
    Ok(AsyncTask::new(ReadJsonTask { file }))
}

#[napi(js_name = "writeJsonSync")]
pub fn write_json_sync(file: String, data: String, options: Option<JsWriteOptions>) -> Result<()> {
    let (mode, flag) = write_options(options);
    json::write_json_sync_impl(&file, data.as_bytes(), mode, &flag)
}

#[napi(js_name = "writeJsonAsync")]
pub fn write_json_async(
    file: String,
    data: String,
    options: Option<JsWriteOptions>,
) -> Result<AsyncTask<WriteJsonTask>> {
    let (mode, flag) = write_options(options);
    Ok(AsyncTask::new(WriteJsonTask { file, data, mode, flag }))
}

fn write_options(options: Option<JsWriteOptions>) -> (u32, String) {
    match options {
        None => (0o666, String::from("w")),
        Some(o) => (o.mode.unwrap_or(0o666), o.flag.unwrap_or_else(|| String::from("w"))),
    }
}

// ---------------------------------------------------------------------------
// pathExists (async) — pathExistsSync stays on graceful-fs existsSync, like
// the source
// ---------------------------------------------------------------------------

pub struct PathExistsTask {
    pub path: String,
}

impl PathExistsTask {
    fn compute_work(&mut self) -> Result<bool> {
        // The source resolves pathExists via fs.access, which succeeds exactly
        // when the path can be stat'ed.
        Ok(std::fs::metadata(&self.path).is_ok())
    }
}

impl_task!(PathExistsTask, bool);

#[napi(js_name = "pathExistsAsync")]
pub fn path_exists_async(path: String) -> Result<AsyncTask<PathExistsTask>> {
    Ok(AsyncTask::new(PathExistsTask { path }))
}

// Re-export the internal test surface as part of the generated binding.
pub use napi_internal::{
    are_identical_js, check_parent_paths_async, check_parent_paths_sync, check_paths_async,
    check_paths_sync, is_src_subdir_js, symlink_paths_async, symlink_paths_sync_js,
    symlink_type_async, symlink_type_sync_js, utimes_millis_async, utimes_millis_sync_native,
};