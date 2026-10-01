// Small path helpers mirroring the node path module for the subset fs-extra needs.
use std::path::{Path, PathBuf};

pub fn dirname(path: &str) -> String {
    let p = Path::new(path);
    let parent = p.parent();
    match parent {
        Some(par) => {
            let s = par.to_string_lossy().to_string();
            // Node's path.dirname('/a') returns '/' and dirname('/') returns '/'.
            if s.is_empty() { '.'.to_string() } else { s }
        }
        None => String::from('.'),
    }
}

/// node path.basename
pub fn basename(path: &str) -> String {
    match Path::new(path) {
        p if p.file_name().is_none() => String::new(),
        p => p.file_name().unwrap().to_string_lossy().to_string(),
    }
}

/// Lexical normalization + absolutization equivalent to node path.resolve on POSIX.
/// Returns paths with the same spelling node would produce (no trailing slash except root).
pub fn resolve(path: &str) -> String {
    let p = path.trim();
    let mut abs = PathBuf::from(p);
    if !abs.is_absolute() {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        abs = cwd.join(abs);
    }
    let mut out = PathBuf::new();
    let mut comps: Vec<std::ffi::OsString> = Vec::new();
    let mut is_abs = false;
    for comp in abs.components() {
        use std::path::Component;
        match comp {
            Component::RootDir => {
                is_abs = true;
            }
            Component::CurDir => {}
            Component::Prefix(_) => out.push(comp.as_os_str()),
            Component::ParentDir => {
                let _ = comps.pop();
            }
            other => comps.push(other.as_os_str().to_os_string()),
        }
    }
    for c in comps {
        out.push(c);
    }
    let s = out.to_string_lossy().to_string();
    if is_abs {
        format!("/{}", s)
    } else if s.is_empty() {
        String::from(".")
    } else {
        s
    }
}

/// node path.parse(p).root — the leading '/' on POSIX.
pub fn root_of(path: &str) -> String {
    if path.starts_with('/') {
        '/'.to_string()
    } else {
        String::new()
    }
}

pub fn is_src_subdir(src: &str, dest: &str) -> bool {
    let sep = '/';
    let src_resolved = resolve(src);
    let dest_resolved = resolve(dest);
    let src_arr: Vec<&str> = src_resolved.split(sep).filter(|i| !i.is_empty()).collect();
    let dest_arr: Vec<&str> = dest_resolved.split(sep).filter(|i| !i.is_empty()).collect();
    src_arr
        .iter()
        .zip(dest_arr.iter())
        .all(|(cur, d)| d == cur)
        && src_arr.len() <= dest_arr.len()
}

/// path.isAbsolute
pub fn is_absolute(path: &str) -> bool {
    path.starts_with('/')
}

/// path.join on POSIX for a few components — drops '.' segments exactly like
/// node (join('.', 'foo.txt') === 'foo.txt'), preserves a leading '/', and
/// handles '..' like node: '..' pops segments of the joined result, and only
/// when all segments are already gone may further '..'s remain in the tail of
/// a relative result (join('a', '../../b') === '../b').
pub fn join2(a: &str, b: &str) -> String {
    let abs = a.starts_with('/');
    let mut segments: Vec<&str> = Vec::new();
    for arg in [a, b] {
        for seg in arg.split('/') {
            if seg.is_empty() || seg == "." {
                continue;
            }
            if seg == ".." {
                match segments.pop() {
                    Some(prev) if prev == ".." => {
                        // we popped a literal '..' (relative tail); push it back
                        segments.push(prev);
                        segments.push("..");
                    }
                    Some(_popped) => {}
                    None => {
                        if abs {
                            // '/..' stays at the root
                        } else {
                            segments.push("..");
                        }
                    }
                }
                continue;
            }
            segments.push(seg);
        }
    }
    let joined = if segments.is_empty() {
        String::new()
    } else {
        segments.join("/")
    };
    if abs {
        format!("/{}", joined)
    } else if joined.is_empty() {
        String::from(".")
    } else {
        joined
    }
}

/// path.relative(from, to) — POSIX, without drive handling.
pub fn relative(from: &str, to: &str) -> String {
    let from_abs = resolve(from);
    let to_abs = resolve(to);
    let from_parts: Vec<&str> = from_abs.split('/').filter(|s| !s.is_empty()).collect();
    let to_parts: Vec<&str> = to_abs.split('/').filter(|s| !s.is_empty()).collect();
    let mut common = 0usize;
    for (f, t) in from_parts.iter().zip(to_parts.iter()) {
        if f == t {
            common += 1;
        } else {
            break;
        }
    }
    let mut res = String::new();
    let up = from_parts.len() - common;
    for _ in 0..up {
        res.push_str("../");
    }
    let rest = to_parts[common..].join("/");
    if rest.is_empty() {
        // node produces '..' / '../..' — the ups without a trailing slash —
        // and '.' when neither side has extra segments
        if res.ends_with('/') {
            res.pop();
        }
        if res.is_empty() {
            res = String::from(".");
        }
        return res;
    }
    res.push_str(&rest);
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve() {
        assert_eq!(resolve("/a/./b/../c"), "/a/c");
        assert_eq!(resolve("/"), "/");
    }

    #[test]
    fn test_join2() {
        assert_eq!(join2(".", "foo.txt"), "foo.txt");
        assert_eq!(join2("empty-dir", "foo.txt"), "empty-dir/foo.txt");
        assert_eq!(join2("dstdir", "../src.txt"), "src.txt");
        assert_eq!(join2("a", "../../b"), "../b");
        assert_eq!(join2("/a", "../b"), "/b");
        assert_eq!(join2("/a/sub", "../b"), "/a/b");
    }

    #[test]
    fn test_is_src_subdir() {
        assert!(is_src_subdir("/a/b", "/a/b/c"));
        assert!(is_src_subdir("/a/b", "/a/b"));
        assert!(!is_src_subdir("/a/b", "/a/bc"));
        assert!(!is_src_subdir("/a/b", "/x"));
        // paths equal after resolution
        assert!(is_src_subdir("/a/./b", "/a/b/c"));
    }

    #[test]
    fn test_relative() {
        assert_eq!(relative("/a/b", "/a/b/c"), "c");
        assert_eq!(relative("/a/b/c", "/a/b"), "..");
        assert_eq!(relative("/a/b/c/d", "/a/b"), "../..");
        assert_eq!(relative("/a/b", "/x/y"), "../../x/y");
        assert_eq!(relative("/a/b", "/a/b"), ".");
    }
}