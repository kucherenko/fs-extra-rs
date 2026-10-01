// Port of node-fs-extra/lib/util/stat.js — internal stat checks shared by copy/move.
use std::io;

use unicode_normalization::UnicodeNormalization;

use crate::errors::{fserr, plain_error};
use crate::paths::{basename, dirname, is_src_subdir, resolve, root_of};
use crate::util_utimes::{Times, Ts};

#[derive(Debug, Clone, Copy, Default)]
pub struct St {
    pub dev: u64,
    pub ino: u64,
    pub mode: u32,
    pub size: u64,
    pub rdev: u64,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub times: Times,
}

impl St {
    pub fn from_meta(m: &std::fs::Metadata) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            St {
                dev: m.dev(),
                ino: m.ino(),
                mode: m.mode() as u32,
                size: m.size(),
                rdev: m.rdev(),
                nlink: m.nlink() as u32,
                uid: m.uid(),
                gid: m.gid(),
                times: Times {
                    atime: Ts { sec: m.atime(), nsec: m.atime_nsec() as i64 },
                    mtime: Ts { sec: m.mtime(), nsec: m.mtime_nsec() as i64 },
                    ctime: Ts { sec: m.ctime(), nsec: m.ctime_nsec() as i64 },
                    birthtime: m
                        .created()
                        .ok()
                        .and_then(|b| b.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| Ts { sec: d.as_secs() as i64, nsec: d.subsec_nanos() as i64 })
                        .unwrap_or_default(),
                },
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            St {
                dev: 0,
                ino: m.file_index().unwrap_or(0) as u64,
                mode: m.permissions().to_mode(),
                size: m.file_size(),
                rdev: 0,
                nlink: m.number_of_links(),
                uid: 0,
                gid: 0,
                times: Times::default(),
            }
        }
    }

    pub fn is_file(&self) -> bool {
        self.mode & (libc::S_IFMT as u32) == libc::S_IFREG as u32
    }

    pub fn is_dir(&self) -> bool {
        self.mode & (libc::S_IFMT as u32) == libc::S_IFDIR as u32
    }

    pub fn is_symlink(&self) -> bool {
        self.mode & (libc::S_IFMT as u32) == libc::S_IFLNK as u32
    }

    pub fn is_char_device(&self) -> bool {
        self.mode & (libc::S_IFMT as u32) == libc::S_IFCHR as u32
    }

    pub fn is_block_device(&self) -> bool {
        self.mode & (libc::S_IFMT as u32) == libc::S_IFBLK as u32
    }

    pub fn is_fifo(&self) -> bool {
        self.mode & (libc::S_IFMT as u32) == libc::S_IFIFO as u32
    }

    pub fn is_socket(&self) -> bool {
        self.mode & (libc::S_IFMT as u32) == libc::S_IFSOCK as u32
    }
}

pub fn lstat(path: &str) -> io::Result<St> {
    std::fs::symlink_metadata(path).map(|m| St::from_meta(&m))
}

pub fn stat(path: &str) -> io::Result<St> {
    std::fs::metadata(path).map(|m| St::from_meta(&m))
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PathCheck {
    pub src_stat: St,
    pub dest_stat: Option<St>,
    pub is_changing_case: bool,
}

// Port of util/stat.js areIdentical: ino and dev are never undefined for real
// stats, so a plain comparison is exact.
pub fn are_identical(src_stat: &St, dest_stat: &St) -> bool {
    src_stat.ino == dest_stat.ino && src_stat.dev == dest_stat.dev
}

fn cosmetic_rename(src_base_name: &str, dest_base_name: &str) -> bool {
    src_base_name.to_lowercase().nfc().collect::<String>()
        == dest_base_name.to_lowercase().nfc().collect::<String>()
}

fn err_msg(src: &str, dest: &str, func_name: &str) -> String {
    format!("Cannot {} '{}' to a subdirectory of itself, '{}'.", func_name, src, dest)
}

fn get_stats(src: &str, dest: &str, dereference: bool) -> Result<(St, Option<St>), napi::Error> {
    let stat_fn = if dereference { stat } else { lstat };
    let src_stat = stat_fn(src).map_err(|e| fserr(&e, if dereference { "stat" } else { "lstat" }, src))?;
    let dest_stat = match stat_fn(dest) {
        Ok(s) => Some(s),
        Err(e) => {
            if e.raw_os_error() == Some(libc::ENOENT) {
                None
            } else {
                return Err(fserr(&e, if dereference { "stat" } else { "lstat" }, dest));
            }
        }
    };
    Ok((src_stat, dest_stat))
}

// Port of util/stat.js checkPaths / checkPathsSync.
pub fn check_paths(
    src: &str,
    dest: &str,
    func_name: &str,
    dereference: bool,
) -> Result<PathCheck, napi::Error> {
    let (src_stat, dest_stat) = get_stats(src, dest, dereference)?;
    if let Some(ref dest_stat) = dest_stat {
        if are_identical(&src_stat, dest_stat) {
            let src_base_name = basename(src);
            let dest_base_name = basename(dest);
            if func_name == "move"
                && src_base_name != dest_base_name
                && cosmetic_rename(&src_base_name, &dest_base_name)
            {
                return Ok(PathCheck { src_stat, dest_stat: Some(*dest_stat), is_changing_case: true });
            }
            return Err(plain_error("Source and destination must not be the same."));
        }
        if src_stat.is_dir() && !dest_stat.is_dir() {
            return Err(plain_error(format!(
                "Cannot overwrite non-directory '{}' with directory '{}'.",
                dest, src
            )));
        }
        if !src_stat.is_dir() && dest_stat.is_dir() {
            return Err(plain_error(format!(
                "Cannot overwrite directory '{}' with non-directory '{}'.",
                dest, src
            )));
        }
    }

    if src_stat.is_dir() && is_src_subdir(src, dest) {
        return Err(plain_error(err_msg(src, dest, func_name)));
    }

    Ok(PathCheck { src_stat, dest_stat, is_changing_case: false })
}

// Port of checkParentPaths / checkParentPathsSync: walks the destination's
// ancestors looking for the source inode.
pub(crate) fn check_parent_paths_impl(
    src: &str,
    src_stat: &St,
    dest: &str,
    func_name: &str,
) -> Result<(), napi::Error> {
    // The recursion in the source passes destParent as the new dest, so the
    // error message reports the walked path (the deepest ancestor inside
    // src), matching the source's messages.
    let src_parent = resolve(&dirname(src));
    let mut cur = dest.to_string();
    loop {
        let dest_parent = resolve(&dirname(&cur));
        if dest_parent == src_parent || dest_parent == root_of(&dest_parent) {
            return Ok(());
        }
        match stat(&dest_parent) {
            Ok(dest_stat) => {
                if are_identical(src_stat, &dest_stat) {
                    return Err(plain_error(err_msg(src, &cur, func_name)));
                }
            }
            Err(err) => {
                // The destination parent does not exist yet, but a deeper
                // ancestor might (e.g. when it is a symlink into the source
                // tree). Keep walking up so the self-subdirectory check is not
                // bypassed.
                if err.raw_os_error() != Some(libc::ENOENT) {
                    return Err(fserr(&err, "stat", &dest_parent));
                }
            }
        }
        let next = resolve(&dirname(&dest_parent));
        if next == dest_parent {
            return Ok(());
        }
        cur = dest_parent;
    }
}

pub fn check_paths_with_parents(
    src: &str,
    dest: &str,
    func_name: &str,
    dereference: bool,
) -> Result<PathCheck, napi::Error> {
    let check = check_paths(src, dest, func_name, dereference)?;
    check_parent_paths_impl(src, &check.src_stat, dest, func_name)?;
    Ok(check)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_paths_same() {
        let dir = std::env::temp_dir().join("rustfs-test-check-paths");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a");
        std::fs::write(&file, "x").unwrap();
        let e = check_paths(file.to_str().unwrap(), file.to_str().unwrap(), "copy", false).unwrap_err();
        assert!(format!("{:?}", e).contains("Source and destination must not be the same."));
        let _ = std::fs::remove_dir_all(&dir);
    }
}