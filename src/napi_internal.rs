use napi_derive::napi;

// Exports mirroring node-fs-extra's internal modules (lib/util/stat.js,
// lib/ensure/symlink-paths.js, symlink-type.js, lib/util/utimes.js) so the
// ported internal tests probe the real implementation. These live on the
// binding but only surface through the target's internal lib/ modules, never
// on the package's public API.
use napi::bindgen_prelude::{AsyncTask, BigInt, ToNapiValue};
use napi::{Env, JsObject, Result, Task};

use crate::ensure::{symlink_paths_async_impl, symlink_paths_sync, symlink_type_sync};
use crate::util_stat::{check_parent_paths_impl, check_paths, PathCheck, St};
use crate::util_utimes::Ts;

#[derive(Debug, Clone)]
pub struct JsStat {
    pub dev: BigInt,
    pub ino: BigInt,
    pub mode: BigInt,
    pub nlink: BigInt,
    pub uid: BigInt,
    pub gid: BigInt,
    pub rdev: BigInt,
    pub size: BigInt,
    pub atime_ms: BigInt,
    pub mtime_ms: BigInt,
    pub ctime_ms: BigInt,
    pub birthtime_ms: BigInt,
}

impl ToNapiValue for JsStat {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        let env = Env::from_raw(env);
        let mut obj = env.create_object()?;
        obj.set("dev", val.dev.clone())?;
        obj.set("ino", val.ino.clone())?;
        obj.set("mode", val.mode.clone())?;
        obj.set("nlink", val.nlink.clone())?;
        obj.set("uid", val.uid.clone())?;
        obj.set("gid", val.gid.clone())?;
        obj.set("rdev", val.rdev.clone())?;
        obj.set("size", val.size.clone())?;
        obj.set("atimeMs", val.atime_ms.clone())?;
        obj.set("mtimeMs", val.mtime_ms.clone())?;
        obj.set("ctimeMs", val.ctime_ms.clone())?;
        obj.set("birthtimeMs", val.birthtime_ms.clone())?;
        // type predicate methods, matching node Stats semantics
        let is_dir = st_is_dir(&val.mode);
        let is_file = st_is_file(&val.mode);
        let is_symlink = st_is_symlink(&val.mode);
        obj.set(
            "isFile",
            env.create_function_from_closure("isFile", move |_ctx| Ok(is_file))?,
        )?;
        obj.set(
            "isDirectory",
            env.create_function_from_closure("isDirectory", move |_ctx| Ok(is_dir))?,
        )?;
        obj.set(
            "isSymbolicLink",
            env.create_function_from_closure("isSymbolicLink", move |_ctx| Ok(is_symlink))?,
        )?;
        ToNapiValue::to_napi_value(env.raw(), obj)
    }
}

fn mode_u64(mode: &BigInt) -> u64 {
    let (sign, val, _lossless) = mode.get_u64();
    if sign { 0 } else { val }
}

fn st_is_file(mode: &BigInt) -> bool {
    mode_u64(mode) & (libc::S_IFMT as u64) == libc::S_IFREG as u64
}

fn st_is_dir(mode: &BigInt) -> bool {
    mode_u64(mode) & (libc::S_IFMT as u64) == libc::S_IFDIR as u64
}

fn st_is_symlink(mode: &BigInt) -> bool {
    mode_u64(mode) & (libc::S_IFMT as u64) == libc::S_IFLNK as u64
}

fn big(env: &Env, v: u64) -> napi::Result<BigInt> {
    let _ = env;
    Ok(BigInt::from(v))
}

fn big_i(env: &Env, v: i64) -> napi::Result<BigInt> {
    let _ = env;
    Ok(BigInt::from(v))
}

fn stat_to_js(env: &Env, st: &St) -> napi::Result<JsStat> {
    Ok(JsStat {
        dev: big(env, st.dev)?,
        ino: big(env, st.ino)?,
        mode: big(env, st.mode as u64)?,
        nlink: big(env, st.nlink as u64)?,
        uid: big(env, st.uid as u64)?,
        gid: big(env, st.gid as u64)?,
        rdev: big(env, st.rdev)?,
        size: big(env, st.size)?,
        atime_ms: big_i(env, st.times.atime.sec * 1_000_000 + st.times.atime.nsec / 1_000_000)?,
        mtime_ms: big_i(env, st.times.mtime.sec * 1_000_000 + st.times.mtime.nsec / 1_000_000)?,
        ctime_ms: big_i(env, st.times.ctime.sec * 1_000_000 + st.times.ctime.nsec / 1_000_000)?,
        birthtime_ms: big_i(env, st.times.birthtime.sec * 1_000_000 + st.times.birthtime.nsec / 1_000_000)?,
    })
}

/// `{ srcStat, destStat, isChangingCase? }` with bigint stats — the shape
/// checkPaths returns in the source (stats are { bigint: true } stats).
pub struct JsPathCheck {
    pub src_stat: Option<JsStatBox>,
    pub dest_stat: Option<JsStatBox>,
    pub is_changing_case: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
pub struct JsStatBox(pub St);

impl ToNapiValue for JsStatBox {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        let env = Env::from_raw(env);
        stat_to_js(&env, &val.0).and_then(|s| ToNapiValue::to_napi_value(env.raw(), s))
    }
}

impl ToNapiValue for JsPathCheck {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        let env = Env::from_raw(env);
        let mut obj = env.create_object()?;
        obj.set("srcStat", val.src_stat)?;
        obj.set("destStat", val.dest_stat)?;
        if let Some(c) = val.is_changing_case {
            obj.set("isChangingCase", c)?;
        }
        ToNapiValue::to_napi_value(env.raw(), obj)
    }
}

fn path_check_to_js(check: PathCheck) -> JsPathCheck {
    JsPathCheck {
        src_stat: Some(JsStatBox(check.src_stat)),
        dest_stat: check.dest_stat.map(JsStatBox),
        is_changing_case: if check.is_changing_case { Some(true) } else { None },
    }
}

// ---------------------------------------------------------------------------
// util/stat.js: checkPaths / checkPathsSync
// ---------------------------------------------------------------------------

pub struct CheckPathsTask {
    pub src: String,
    pub dest: String,
    pub func_name: String,
}

impl Task for CheckPathsTask {
    type Output = PathCheck;
    type JsValue = JsPathCheck;
    fn compute(&mut self) -> Result<PathCheck> {
        check_paths(&self.src, &self.dest, &self.func_name, false)
    }
    fn resolve(&mut self, _env: Env, output: PathCheck) -> Result<JsPathCheck> {
        Ok(path_check_to_js(output))
    }
}

#[napi(js_name = "checkPathsSync")]
pub fn check_paths_sync(_env: Env, src: String, dest: String, func_name: String) -> Result<JsPathCheck> {
    let check = check_paths(&src, &dest, &func_name, false)?;
    Ok(path_check_to_js(check))
}

#[napi(js_name = "checkPathsAsync")]
pub fn check_paths_async(src: String, dest: String, func_name: String) -> Result<AsyncTask<CheckPathsTask>> {
    Ok(AsyncTask::new(CheckPathsTask { src, dest, func_name }))
}

// ---------------------------------------------------------------------------
// util/stat.js: checkParentPaths / checkParentPathsSync
// ---------------------------------------------------------------------------

/// ino/dev of a stat-like JS object with JS === type fidelity (bigint and
/// number are distinct identities).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IdKind {
    Number,
    BigInt,
}

impl napi::bindgen_prelude::TypeName for JsStatBox {
    fn type_name() -> &'static str {
        "Object"
    }

    fn value_type() -> napi::ValueType {
        napi::ValueType::Object
    }
}

impl napi::bindgen_prelude::TypeName for JsPathCheck {
    fn type_name() -> &'static str {
        "Object"
    }

    fn value_type() -> napi::ValueType {
        napi::ValueType::Object
    }
}

fn stat_ids_of(obj: &JsObject) -> Result<Option<(IdKind, u64, u64)>> {
    let ino = obj.get::<&str, napi::JsUnknown>("ino")?;
    let dev = obj.get::<&str, napi::JsUnknown>("dev")?;
    let (ino, dev) = match (ino, dev) {
        (Some(i), Some(d)) => (i, d),
        _ => return Ok(None),
    };
    match (num_id(&ino)?, num_id(&dev)?) {
        (Some(i), Some(d)) if i.0 == d.0 => Ok(Some((i.0, i.1, d.1))),
        _ => Ok(None),
    }
}

fn num_id(v: &napi::JsUnknown) -> Result<Option<(IdKind, u64)>> {
    match v.get_type()? {
        napi::ValueType::BigInt => {
            let big: napi::JsBigInt = unsafe { v.cast() };
            let (val, _lossless) = big.get_u64()?;
            Ok(Some((IdKind::BigInt, val)))
        }
        napi::ValueType::Number => {
            let number: napi::JsNumber = unsafe { v.cast() };
            let d = number.get_double()?;
            Ok(Some((IdKind::Number, d as u64)))
        }
        _ => Ok(None),
    }
}

pub struct CheckParentPathsTask {
    pub src: String,
    pub src_stat: Option<(IdKind, u64, u64)>,
    pub dest: String,
    pub func_name: String,
}

impl Task for CheckParentPathsTask {
    type Output = ();
    type JsValue = ();
    fn compute(&mut self) -> Result<()> {
        check_parent_paths_by_id(&self.src, self.src_stat, &self.dest, &self.func_name)
    }
    fn resolve(&mut self, _env: Env, output: ()) -> Result<()> {
        Ok(output)
    }
}

fn check_parent_paths_by_id(
    src: &str,
    src_ids: Option<(IdKind, u64, u64)>,
    dest: &str,
    func_name: &str,
) -> Result<()> {
    let (_kind, ino, dev) = match src_ids {
        Some(ids) => ids,
        None => return Ok(()),
    };
    let src_stat = St { dev, ino, ..St::default() };
    check_parent_paths_impl(src, &src_stat, dest, func_name)
}

#[napi(js_name = "checkParentPathsSync")]
pub fn check_parent_paths_sync(
    src: String,
    src_stat: JsObject,
    dest: String,
    func_name: String,
) -> Result<()> {
    check_parent_paths_by_id(&src, stat_ids_of(&src_stat)?, &dest, &func_name)
}

#[napi(js_name = "checkParentPathsAsync")]
pub fn check_parent_paths_async(
    src: String,
    src_stat: JsObject,
    dest: String,
    func_name: String,
) -> Result<AsyncTask<CheckParentPathsTask>> {
    Ok(AsyncTask::new(CheckParentPathsTask {
        src,
        src_stat: stat_ids_of(&src_stat)?,
        dest,
        func_name,
    }))
}

// ---------------------------------------------------------------------------
// util/stat.js: areIdentical / isSrcSubdir (test-facing)
// ---------------------------------------------------------------------------

/// Port of util/stat.js areIdentical for test-facing use: two stat-like
/// objects compared with JS === semantics (bigint never equals number).
#[napi(js_name = "areIdenticalJs")]
pub fn are_identical_js(src_stat: JsObject, dest_stat: JsObject) -> Result<bool> {
    Ok(match (stat_ids_of(&src_stat)?, stat_ids_of(&dest_stat)?) {
        (Some(a), Some(b)) => a.0 == b.0 && a.1 == b.1 && a.2 == b.2,
        _ => false,
    })
}

/// Port of util/stat.js isSrcSubdir for test-facing use.
#[napi(js_name = "isSrcSubdirJs")]
pub fn is_src_subdir_js(src: String, dest: String) -> Result<bool> {
    Ok(crate::paths::is_src_subdir(&src, &dest))
}

// ---------------------------------------------------------------------------
// ensure/symlink-paths.js: symlinkPaths / symlinkPathsSync
// ---------------------------------------------------------------------------

#[napi(object)]
#[derive(Debug, Default)]
pub struct JsSymlinkPaths {
    pub to_cwd: String,
    pub to_dst: String,
}

pub struct SymlinkPathsTask {
    pub srcpath: String,
    pub dstpath: String,
}

impl Task for SymlinkPathsTask {
    type Output = JsSymlinkPaths;
    type JsValue = JsSymlinkPaths;
    fn compute(&mut self) -> Result<JsSymlinkPaths> {
        symlink_paths_async_impl(&self.srcpath, &self.dstpath).map(|(tc, td)| JsSymlinkPaths { to_cwd: tc, to_dst: td })
    }
    fn resolve(&mut self, _env: Env, output: JsSymlinkPaths) -> Result<JsSymlinkPaths> {
        Ok(output)
    }
}

#[napi(js_name = "symlinkPathsSync")]
pub fn symlink_paths_sync_js(srcpath: String, dstpath: String) -> Result<JsSymlinkPaths> {
    let (to_cwd, to_dst) = symlink_paths_sync(&srcpath, &dstpath)?;
    Ok(JsSymlinkPaths { to_cwd, to_dst })
}

#[napi(js_name = "symlinkPathsAsync")]
pub fn symlink_paths_async(
    srcpath: String,
    dstpath: String,
) -> Result<AsyncTask<SymlinkPathsTask>> {
    Ok(AsyncTask::new(SymlinkPathsTask { srcpath, dstpath }))
}

// ---------------------------------------------------------------------------
// ensure/symlink-type.js: symlinkType / symlinkTypeSync
// ---------------------------------------------------------------------------

pub struct SymlinkTypeTask {
    pub srcpath: String,
    pub symlink_type: Option<String>,
}

impl Task for SymlinkTypeTask {
    type Output = String;
    type JsValue = String;
    fn compute(&mut self) -> Result<String> {
        symlink_type_sync(&self.srcpath, self.symlink_type.as_deref())
    }
    fn resolve(&mut self, _env: Env, output: String) -> Result<String> {
        Ok(output)
    }
}

#[napi(js_name = "symlinkTypeSync")]
pub fn symlink_type_sync_js(srcpath: String, _type: Option<String>) -> Result<String> {
    symlink_type_sync(&srcpath, _type.as_deref())
}

#[napi(js_name = "symlinkTypeAsync")]
pub fn symlink_type_async(
    srcpath: String,
    _type: Option<String>,
) -> Result<AsyncTask<SymlinkTypeTask>> {
    Ok(AsyncTask::new(SymlinkTypeTask { srcpath, symlink_type: _type }))
}

// ---------------------------------------------------------------------------
// util/utimes.js: utimesMillis / utimesMillisSync (times arrive as epoch ms,
// pre-normalized by the JS glue implementing node's toUnixTimestamp)
// ---------------------------------------------------------------------------

pub struct UtimesMillisTask {
    pub path: String,
    pub atime_ms: f64,
    pub mtime_ms: f64,
}

impl Task for UtimesMillisTask {
    type Output = ();
    type JsValue = ();
    fn compute(&mut self) -> Result<()> {
        crate::util_utimes::utimes_millis_sync_impl(
            &self.path,
            Ts::new(self.atime_ms),
            Ts::new(self.mtime_ms),
        )
    }
    fn resolve(&mut self, _env: Env, output: ()) -> Result<()> {
        Ok(output)
    }
}

#[napi(js_name = "utimesMillisSyncNative")]
pub fn utimes_millis_sync_native(path: String, atime_ms: f64, mtime_ms: f64) -> Result<()> {
    crate::util_utimes::utimes_millis_sync_impl(&path, Ts::new(atime_ms), Ts::new(mtime_ms))
}

#[napi(js_name = "utimesMillisAsync")]
pub fn utimes_millis_async(
    path: String,
    atime_ms: f64,
    mtime_ms: f64,
) -> Result<AsyncTask<UtimesMillisTask>> {
    Ok(AsyncTask::new(UtimesMillisTask { path, atime_ms, mtime_ms }))
}