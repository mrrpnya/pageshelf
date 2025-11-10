//! Extensions module
//!
//! Extends various existing datatypes with new features for ease of use.

use std::path::{Component, Path, PathBuf};

/// Denotes a type that may be normalized (simplified) in some way
pub trait Normalizable {
    /// The type it should normalize to.
    type Output;
    /// Returns a normalized path with a leading slash.
    fn normalized_absolute(&self) -> Self::Output;

    /// Returns a normalized path without a leading slash.
    fn normalized_relative(&self) -> Self::Output;
}

impl Normalizable for Path {
    type Output = PathBuf;

    fn normalized_absolute(&self) -> Self::Output {
        let mut normalized = PathBuf::new();
        let mut saw_root = false;

        for component in self.components() {
            match component {
                Component::CurDir => {
                    if !saw_root && normalized.as_os_str().is_empty() {
                        normalized.push("/");
                        saw_root = true;
                    }
                }
                Component::ParentDir => {
                    normalized.pop();
                }
                Component::RootDir => {
                    if !saw_root {
                        normalized.push("/");
                        saw_root = true;
                    }
                }
                _ => normalized.push(component.as_os_str()),
            }
        }

        if !saw_root {
            let mut with_root = PathBuf::from("/");
            with_root.push(&normalized);
            normalized = with_root;
        }

        normalized
    }

    fn normalized_relative(&self) -> Self::Output {
        let abs_normalized = self.normalized_absolute();
        let abs_str = abs_normalized.to_str().unwrap_or("");

        // Strip leading slash if present
        if let Some(stripped) = abs_str.strip_prefix('/') {
            PathBuf::from(stripped)
        } else {
            abs_normalized
        }
    }
}

impl Normalizable for PathBuf {
    type Output = PathBuf;

    fn normalized_absolute(&self) -> Self::Output {
        self.as_path().normalized_absolute()
    }

    fn normalized_relative(&self) -> Self::Output {
        self.as_path().normalized_relative()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_normalized_simple() {
        let path = Path::new("/foo/bar");
        assert_eq!(path.normalized_absolute(), Path::new("/foo/bar"));
    }

    #[test]
    fn test_normalized_with_current_dir() {
        let path = Path::new("/foo/./bar");
        assert_eq!(path.normalized_absolute(), Path::new("/foo/bar"));
    }

    #[test]
    fn test_normalized_with_parent_dir() {
        let path = Path::new("/foo/bar/../baz");
        assert_eq!(path.normalized_absolute(), Path::new("/foo/baz"));
    }

    #[test]
    fn test_normalized_with_multiple_parent_dirs() {
        let path = Path::new("/foo/bar/baz/../../qux");
        assert_eq!(path.normalized_absolute(), Path::new("/foo/qux"));
    }

    #[test]
    fn test_normalized_only_current_dir() {
        let path = Path::new("./.");
        assert_eq!(path.normalized_absolute(), Path::new("/"));
    }

    #[test]
    fn test_normalized_only_parent_dirs() {
        let path = Path::new("../../foo");
        assert_eq!(path.normalized_absolute(), Path::new("/foo"));
    }

    #[test]
    fn test_normalized_root_only() {
        let path = Path::new("/");
        assert_eq!(path.normalized_absolute(), Path::new("/"));
    }
}

#[cfg(test)]
mod relative_tests {
    use super::*;

    #[test]
    fn test_normalized_relative_simple() {
        let path = Path::new("/foo/bar");
        assert_eq!(path.normalized_relative(), Path::new("foo/bar"));
    }

    #[test]
    fn test_normalized_relative_with_parent_dir() {
        let path = Path::new("/foo/bar/../baz");
        assert_eq!(path.normalized_relative(), Path::new("foo/baz"));
    }

    #[test]
    fn test_normalized_relative_root_only() {
        let path = Path::new("/");
        assert_eq!(path.normalized_relative(), Path::new(""));
    }
}
