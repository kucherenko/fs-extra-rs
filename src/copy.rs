// Port of node-fs-extra/lib/copy/copy.js — asynchronous copy.
//
// The whole algorithm runs natively on the libuv threadpool. The optional
// `filter` function is invoked on the main thread through a threadsafe
// function (the JS glue adapts it to the (err, src, dest) callback shape and
// always returns a promise), awaited with a parking block_on. Directory
// items are processed concurrently with rayon, matching the source's
// concurrent per-item processing.
use rayon::prelude::*;
use std::future::Future;
use std::io;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use napi::bindgen_prelude::Promise;
use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction};

use crate::copy_sync::{copy_file, join_pair, readlink_sync, unlink_sync};
pub(crate) use crate::copy_sync::CopyOpts;
use crate::ensure::symlink_sync;
use crate::errors::{fserr, plain_error};
use crate::mkdirs::make_dir_impl;
use crate::paths::{is_src_subdir, resolve};
use crate::util_stat::{check_paths, check_parent_paths_impl, lstat, stat};

/// A parking block_on for futures driven from blocking worker threads.
pub fn block_on<F: Future>(fut: F) -> F::Output {
    struct ThreadWaker(std::thread::Thread);
    impl Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark()
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark()
        }
    }
    let mut fut = Box::pin(fut);
    let waker: Waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => std::thread::park(),
        }
    }
}

pub type FilterTsfn = ThreadsafeFunction<(String, String), ErrorStrategy::CalleeHandled>;

/// The filter runner: per item the copy awaits (possibly promise-returning)
/// filter results, mirroring `include = await runFilter(...)` of the source.
#[derive(Clone)]
pub enum Filter {
    None,
    Tsfn(Arc<FilterTsfn>),
}

impl Filter {
    pub async fn run(&self, src: String, dest: String) -> Result<bool, napi::Error> {
        match self {
            Filter::None => Ok(true),
            Filter::Tsfn(tsfn) => {
                // The adapter (JS glue) turns the user filter into
                // (err, src, dest) => Promise.resolve(!!filter(src, dest)), so
                // the returned value is a promise of a boolean include answer.
                let promise = tsfn
                    .call_async::<Promise<bool>>(Ok((src, dest)))
                    .await?;
                Ok(promise.await?)
            }
        }
    }
}

/// Port of the async `copy(src, dest, opts)` function.
pub fn copy_dispatch(src: String, dest: String, opts: CopyOpts, filter: Filter) -> Result<(), napi::Error> {
    // NOTE: the source emits the fs-extra-WARN0001 warning on 32-bit node for
    // preserveTimestamps; not applicable to this build.
    async_copy(src, dest, opts, filter)
}

pub(crate) fn async_copy(src: String, dest: String, opts: CopyOpts, filter: Filter) -> Result<(), napi::Error> {
    // const { srcStat, destStat } = await stat.checkPaths(src, dest, 'copy', opts)
    let check = check_paths(&src, &dest, "copy", opts.dereference)?;

    // await stat.checkParentPaths(src, srcStat, dest, 'copy')
    check_parent_paths_impl(&src, &check.src_stat, &dest, "copy")?;

    // const include = await runFilter(src, dest, opts); if (!include) return
    let include = block_on(filter.run(src.clone(), dest.clone()))?;
    if !include {
        return Ok(());
    }

    // check if the parent of dest exists, and create it if it doesn't exist
    let dest_parent = crate::paths::dirname(&dest);
    if std::fs::metadata(&dest_parent).is_err() {
        make_dir_impl(&dest_parent, None)?;
    }

    get_stats_and_perform_copy(check.dest_stat, &src, &dest, &opts, &filter)
}

fn get_stats_and_perform_copy(
    dest_stat: Option<crate::util_stat::St>,
    src: &str,
    dest: &str,
    opts: &CopyOpts,
    filter: &Filter,
) -> Result<(), napi::Error> {
    let src_stat = if opts.dereference {
        stat(src).map_err(|e| fserr(&e, "stat", src))?
    } else {
        lstat(src).map_err(|e| fserr(&e, "lstat", src))?
    };

    if src_stat.is_dir() {
        return on_dir(src_stat, dest_stat, src, dest, opts, filter);
    }
    if src_stat.is_file() || src_stat.is_char_device() || src_stat.is_block_device() {
        return on_file(src_stat, dest_stat, src, dest, opts);
    }
    if src_stat.is_symlink() {
        return on_link(dest_stat, src, dest, opts);
    }
    if src_stat.is_socket() {
        return Err(plain_error(format!("Cannot copy a socket file: {}", src)));
    }
    if src_stat.is_fifo() {
        return Err(plain_error(format!("Cannot copy a FIFO pipe: {}", src)));
    }
    Err(plain_error(format!("Unknown file: {}", src)))
}

fn on_file(
    src_stat: crate::util_stat::St,
    dest_stat: Option<crate::util_stat::St>,
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
    src_stat: crate::util_stat::St,
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

fn on_dir(
    src_stat: crate::util_stat::St,
    dest_stat: Option<crate::util_stat::St>,
    src: &str,
    dest: &str,
    opts: &CopyOpts,
    filter: &Filter,
) -> Result<(), napi::Error> {
    if dest_stat.is_none() {
        return mk_dir_and_copy(src_stat, src, dest, opts, filter);
    }
    copy_dir(src, dest, opts, filter)
}

fn mk_dir_and_copy(
    src_stat: crate::util_stat::St,
    src: &str,
    dest: &str,
    opts: &CopyOpts,
    filter: &Filter,
) -> Result<(), napi::Error> {
    mkdir_bare(dest)?;
    copy_dir(src, dest, opts, filter)?;
    crate::copy_sync::chmod_sync(dest, src_stat.mode & 0o7777)
}

/// The async equivalent of the source's onDir iteration: readdir once, then
/// process items concurrently (the source schedules per-item promises via
/// asyncIteratorConcurrentProcess). Errors surface with the first (by
/// directory order) winner, like Promise.all over the per-item futures.
fn copy_dir(
    src: &str,
    dest: &str,
    opts: &CopyOpts,
    filter: &Filter,
) -> Result<(), napi::Error> {
    let entries = std::fs::read_dir(src).map_err(|e| fserr(&e, "opendir", src))?;
    let mut items: Vec<String> = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => return Err(fserr(&e, "readdir", src)),
        };
        items.push(entry.file_name().to_string_lossy().into_owned());
    }

    let errors: Vec<(usize, napi::Error)> = items
        .into_par_iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let (src_item, dest_item) = join_pair(src, dest, &item);
            let work = || -> Result<(), napi::Error> {
                let include = block_on(filter.run(src_item.clone(), dest_item.clone()))?;
                if !include {
                    return Ok(());
                }
                let check = check_paths(&src_item, &dest_item, "copy", opts.dereference)?;
                get_stats_and_perform_copy(check.dest_stat, &src_item, &dest_item, opts, filter)
            };
            work().err().map(|e| (index, e))
        })
        .collect();

    if let Some((_, first)) = errors.into_iter().min_by_key(|(i, _)| *i) {
        return Err(first);
    }
    Ok(())
}

fn mkdir_bare(dest: &str) -> Result<(), napi::Error> {
    let e = |err: &io::Error| fserr(err, "mkdir", dest);
    let c = match std::ffi::CString::new(dest.as_bytes().to_vec()) {
        Ok(c) => c,
        Err(_) => {
            let err = io::Error::from_raw_os_error(libc::EINVAL);
            return Err(e(&err));
        }
    };
    let rc = unsafe { libc::mkdir(c.as_ptr(), 0o777 as libc::mode_t) };
    if rc == 0 {
        Ok(())
    } else {
        let err = io::Error::last_os_error();
        Err(e(&err))
    }
}

fn on_link(
    dest_stat: Option<crate::util_stat::St>,
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

    let mut resolved_dest: String;
    match readlink_sync(dest) {
        Ok(r) => resolved_dest = r,
        Err(err) => {
            // dest exists and is a regular file or directory; if dest already
            // exists, fs throws error anyway
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