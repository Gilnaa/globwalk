// Copyright (c) 2017 Gilad Naaman
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
//! Recursively find files in a directory using globs.
//!
//! Features include
//! - [`gitignore`'s extended glob syntax][gitignore]
//! - Control over symlink behavior
//! - Control depth walked
//! - Control order results are returned
//!
//! [gitignore]: https://git-scm.com/docs/gitignore#_pattern_format
//!
//! # Examples
//!
//! ## Finding image files in the current directory.
//!
//! ```no_run
//! extern crate globwalk;
//!
//! use std::fs;
//!
//! for img in globwalk::glob("*.{png,jpg,gif}").unwrap() {
//!     if let Ok(img) = img {
//!         fs::remove_file(img.path()).unwrap();
//!     }
//! }
//! ```
//!
//! ## Tweak walk options ###
//!
//! ```rust,no_run
//! extern crate globwalk;
//!
//! use std::fs;
//!
//! let walker = globwalk::glob("*.{png,jpg,gif}")
//!     .unwrap()
//!     .max_depth(4)
//!     .follow_links(true)
//!     .into_iter()
//!     .filter_map(Result::ok);
//! for img in walker {
//!     fs::remove_file(img.path()).unwrap();
//! }
//! ```
//!
//! ## Advanced Globbing ###
//!
//! By using one of the constructors of `globwalk::GlobWalker`, it is possible to alter the
//! base-directory or add multiple patterns.
//!
//! ```rust,no_run
//! extern crate globwalk;
//!
//! use std::fs;
//!
//! # let BASE_DIR = ".";
//! let walker = globwalk::GlobWalker::from_patterns(BASE_DIR, &["*.{png,jpg,gif}", "!Pictures/*"])
//!     .unwrap()
//!     .into_iter()
//!     .filter_map(Result::ok);
//!
//! for img in walker {
//!     fs::remove_file(img.path()).unwrap();
//! }
//! ```

extern crate ignore;
extern crate walkdir;

#[cfg(test)]
extern crate tempdir;

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::Match;
use std::cmp::Ordering;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

/// Error from parsing globs.
pub type GlobError = ignore::Error;
/// Error from iterating on files.
pub type WalkError = walkdir::Error;

/// An iterator for recursively yielding glob matches.
///
/// The order of elements yielded by this iterator is unspecified.
pub struct GlobWalker {
    ignore: Gitignore,
    walker: WalkDir,
}

impl GlobWalker {
    /// Construct a new `GlobWalker` with a glob pattern.
    ///
    /// When iterated, the `base` directory will be recursively searched for paths
    /// matching `pattern`.
    pub fn new<P, S>(base: P, pattern: S) -> Result<Self, GlobError>
    where
        P: AsRef<Path>,
        S: AsRef<str>,
    {
        GlobWalker::from_patterns(base, &[pattern])
    }

    /// Construct a new `GlobWalker` from a list of patterns.
    ///
    /// When iterated, the `base` directory will be recursively searched for paths
    /// matching `patterns`.
    pub fn from_patterns<P, S>(base: P, patterns: &[S]) -> Result<Self, GlobError>
    where
        P: AsRef<Path>,
        S: AsRef<str>,
    {
        let mut builder = GitignoreBuilder::new(base.as_ref());
        for pattern in patterns {
            builder.add_line(None, pattern.as_ref())?;
        }
        let ignore = builder.build()?;

        Ok(Self::from_ignore(ignore))
    }

    /// Construct a new `GlobWalker` from a GlobSet
    ///
    /// When iterated, the base directory will be recursively searched for paths
    /// matching `glob`.
    fn from_ignore(ignore: Gitignore) -> Self {
        GlobWalker {
            walker: WalkDir::new(ignore.path()),
            ignore,
        }
    }
    /// Set the minimum depth of entries yielded by the iterator.
    ///
    /// The smallest depth is `0` and always corresponds to the path given
    /// to the `new` function on this type. Its direct descendents have depth
    /// `1`, and their descendents have depth `2`, and so on.
    pub fn min_depth(mut self, depth: usize) -> Self {
        self.walker = self.walker.min_depth(depth);
        self
    }

    /// Set the maximum depth of entries yield by the iterator.
    ///
    /// The smallest depth is `0` and always corresponds to the path given
    /// to the `new` function on this type. Its direct descendents have depth
    /// `1`, and their descendents have depth `2`, and so on.
    ///
    /// Note that this will not simply filter the entries of the iterator, but
    /// it will actually avoid descending into directories when the depth is
    /// exceeded.
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.walker = self.walker.max_depth(depth);
        self
    }

    /// Follow symbolic links. By default, this is disabled.
    ///
    /// When `yes` is `true`, symbolic links are followed as if they were
    /// normal directories and files. If a symbolic link is broken or is
    /// involved in a loop, an error is yielded.
    ///
    /// When enabled, the yielded [`DirEntry`] values represent the target of
    /// the link while the path corresponds to the link. See the [`DirEntry`]
    /// type for more details.
    ///
    /// [`DirEntry`]: struct.DirEntry.html
    pub fn follow_links(mut self, yes: bool) -> Self {
        self.walker = self.walker.follow_links(yes);
        self
    }

    /// Set the maximum number of simultaneously open file descriptors used
    /// by the iterator.
    ///
    /// `n` must be greater than or equal to `1`. If `n` is `0`, then it is set
    /// to `1` automatically. If this is not set, then it defaults to some
    /// reasonably low number.
    ///
    /// This setting has no impact on the results yielded by the iterator
    /// (even when `n` is `1`). Instead, this setting represents a trade off
    /// between scarce resources (file descriptors) and memory. Namely, when
    /// the maximum number of file descriptors is reached and a new directory
    /// needs to be opened to continue iteration, then a previous directory
    /// handle is closed and has its unyielded entries stored in memory. In
    /// practice, this is a satisfying trade off because it scales with respect
    /// to the *depth* of your file tree. Therefore, low values (even `1`) are
    /// acceptable.
    ///
    /// Note that this value does not impact the number of system calls made by
    /// an exhausted iterator.
    ///
    /// # Platform behavior
    ///
    /// On Windows, if `follow_links` is enabled, then this limit is not
    /// respected. In particular, the maximum number of file descriptors opened
    /// is proportional to the depth of the directory tree traversed.
    pub fn max_open(mut self, n: usize) -> Self {
        self.walker = self.walker.max_open(n);
        self
    }

    /// Set a function for sorting directory entries.
    ///
    /// If a compare function is set, the resulting iterator will return all
    /// paths in sorted order. The compare function will be called to compare
    /// entries from the same directory.
    pub fn sort_by<F>(mut self, cmp: F) -> Self
    where
        F: FnMut(&DirEntry, &DirEntry) -> Ordering + Send + Sync + 'static,
    {
        self.walker = self.walker.sort_by(cmp);
        self
    }

    /// Yield a directory's contents before the directory itself. By default,
    /// this is disabled.
    ///
    /// When `yes` is `false` (as is the default), the directory is yielded
    /// before its contents are read. This is useful when, e.g. you want to
    /// skip processing of some directories.
    ///
    /// When `yes` is `true`, the iterator yields the contents of a directory
    /// before yielding the directory itself. This is useful when, e.g. you
    /// want to recursively delete a directory.
    pub fn contents_first(mut self, yes: bool) -> Self {
        self.walker = self.walker.contents_first(yes);
        self
    }
}

impl IntoIterator for GlobWalker {
    type Item = Result<DirEntry, WalkError>;
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            ignore: self.ignore,
            walker: self.walker.into_iter(),
        }
    }
}

/// An iterator which emits glob-matched patterns.
///
/// An instance of this type must be constructed through `GlobWalker`,
/// which uses a builder-style pattern.
///
/// The order of the yielded paths is undefined, unless specified by the user
/// using `GlobWalker::sort_by`.
pub struct IntoIter {
    ignore: Gitignore,
    walker: walkdir::IntoIter,
}

impl Iterator for IntoIter {
    type Item = Result<DirEntry, WalkError>;

    // Possible optimization - Do not descend into directory that will never be a match
    fn next(&mut self) -> Option<Self::Item> {
        for entry in &mut self.walker {
            match entry {
                Ok(e) => {
                    // Strip the common base directory so that the matcher will be
                    // able to recognize the file name.
                    // `unwrap` here is safe, since walkdir returns the files with relation
                    // to the given base-dir.
                    let is_requested = {
                        let rel_path = e.path().strip_prefix(self.ignore.path()).unwrap();
                        let is_dir = e.file_type().is_dir();
                        match self.ignore.matched_path_or_any_parents(rel_path, is_dir) {
                            Match::None | Match::Whitelist(_) => false,
                            Match::Ignore(_) => true,
                        }
                    };

                    if is_requested {
                        return Some(Ok(e));
                    }
                }
                Err(e) => {
                    return Some(Err(e));
                }
            }
        }

        None
    }
}

/// Construct a new `GlobWalker` with a glob pattern.
///
/// When iterated, the current directory will be recursively searched for paths
/// matching `pattern`.
pub fn glob<S: AsRef<str>>(pattern: S) -> Result<GlobWalker, GlobError> {
    GlobWalker::new(".", pattern)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, File};
    use tempdir::TempDir;

    fn touch(dir: &TempDir, names: &[&str]) {
        for name in names {
            let name = normalize_path_sep(name);
            File::create(dir.path().join(name)).expect("Failed to create a test file");
        }
    }

    fn normalize_path_sep<S: AsRef<str>>(s: S) -> String {
        s.as_ref()
            .replace("[/]", if cfg!(windows) { "\\" } else { "/" })
    }

    #[test]
    fn test_new() {
        let dir = TempDir::new("globset_walkdir").expect("Failed to create temporary folder");
        let dir_path = dir.path();

        touch(&dir, &["a.rs", "a.jpg", "a.png", "b.docx"][..]);

        let mut expected = vec!["a.jpg", "a.png"];

        for matched_file in GlobWalker::new(dir_path, "*.{png,jpg,gif}")
            .unwrap()
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = matched_file
                .path()
                .strip_prefix(dir_path)
                .unwrap()
                .to_str()
                .unwrap();
            let path = normalize_path_sep(path);

            let del_idx = if let Some(idx) = expected.iter().position(|n| &path == n) {
                idx
            } else {
                panic!("Iterated file is unexpected: {}", path);
            };
            expected.remove(del_idx);
        }

        let empty: &[&str] = &[][..];
        assert_eq!(expected, empty);
    }

    #[test]
    fn test_from_patterns() {
        let dir = TempDir::new("globset_walkdir").expect("Failed to create temporary folder");
        let dir_path = dir.path();
        create_dir_all(dir_path.join("src/some_mod")).expect("");
        create_dir_all(dir_path.join("tests")).expect("");
        create_dir_all(dir_path.join("contrib")).expect("");

        touch(
            &dir,
            &[
                "a.rs",
                "b.rs",
                "avocado.rs",
                "lib.c",
                "src[/]hello.rs",
                "src[/]world.rs",
                "src[/]some_mod[/]unexpected.rs",
                "src[/]cruel.txt",
                "contrib[/]README.md",
                "contrib[/]README.rst",
                "contrib[/]lib.rs",
            ][..],
        );

        let mut expected: Vec<_> = [
            "src[/]some_mod[/]unexpected.rs",
            "src[/]world.rs",
            "src[/]hello.rs",
            "lib.c",
            "contrib[/]lib.rs",
            "contrib[/]README.md",
            "contrib[/]README.rst",
        ].iter()
            .map(normalize_path_sep)
            .collect();

        let patterns = ["src/**/*.rs", "*.c", "**/lib.rs", "**/*.{md,rst}"];
        for matched_file in GlobWalker::from_patterns(dir_path, &patterns)
            .unwrap()
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = matched_file
                .path()
                .strip_prefix(dir_path)
                .unwrap()
                .to_str()
                .unwrap();
            let path = normalize_path_sep(path);

            let del_idx = if let Some(idx) = expected.iter().position(|n| &path == n) {
                idx
            } else {
                panic!("Iterated file is unexpected: {}", path);
            };
            expected.remove(del_idx);
        }

        let empty: &[&str] = &[][..];
        assert_eq!(expected, empty);
    }

    #[test]
    fn test_blacklist() {
        let dir = TempDir::new("globset_walkdir").expect("Failed to create temporary folder");
        let dir_path = dir.path();
        create_dir_all(dir_path.join("src/some_mod")).expect("");
        create_dir_all(dir_path.join("tests")).expect("");
        create_dir_all(dir_path.join("contrib")).expect("");

        touch(
            &dir,
            &[
                "a.rs",
                "b.rs",
                "avocado.rs",
                "lib.c",
                "src[/]hello.rs",
                "src[/]world.rs",
                "src[/]some_mod[/]unexpected.rs",
                "src[/]cruel.txt",
                "contrib[/]README.md",
                "contrib[/]README.rst",
                "contrib[/]lib.rs",
            ][..],
        );

        let mut expected: Vec<_> = [
            "src[/]some_mod[/]unexpected.rs",
            "src[/]hello.rs",
            "lib.c",
            "contrib[/]lib.rs",
            "contrib[/]README.md",
            "contrib[/]README.rst",
        ].iter()
            .map(normalize_path_sep)
            .collect();

        let patterns = [
            "src/**/*.rs",
            "*.c",
            "**/lib.rs",
            "**/*.{md,rst}",
            "!world.rs",
        ];
        for matched_file in GlobWalker::from_patterns(dir_path, &patterns)
            .unwrap()
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = matched_file
                .path()
                .strip_prefix(dir_path)
                .unwrap()
                .to_str()
                .unwrap();
            let path = normalize_path_sep(path);

            let del_idx = if let Some(idx) = expected.iter().position(|n| &path == n) {
                idx
            } else {
                panic!("Iterated file is unexpected: {}", path);
            };
            expected.remove(del_idx);
        }

        let empty: &[&str] = &[][..];
        assert_eq!(expected, empty);
    }

    #[test]
    fn test_blacklist_dir() {
        let dir = TempDir::new("globset_walkdir").expect("Failed to create temporary folder");
        let dir_path = dir.path();
        create_dir_all(dir_path.join("Pictures")).expect("");

        touch(
            &dir,
            &[
                "a.png",
                "b.png",
                "c.png",
                "Pictures[/]a.png",
                "Pictures[/]b.png",
                "Pictures[/]c.png",
            ][..],
        );

        let mut expected: Vec<_> = ["a.png", "b.png", "c.png"]
            .iter()
            .map(normalize_path_sep)
            .collect();

        let patterns = ["*.{png,jpg,gif}", "!Pictures"];
        for matched_file in GlobWalker::from_patterns(dir_path, &patterns)
            .unwrap()
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = matched_file
                .path()
                .strip_prefix(dir_path)
                .unwrap()
                .to_str()
                .unwrap();
            let path = normalize_path_sep(path);

            let del_idx = if let Some(idx) = expected.iter().position(|n| &path == n) {
                idx
            } else {
                panic!("Iterated file is unexpected: {}", path);
            };
            expected.remove(del_idx);
        }

        let empty: &[&str] = &[][..];
        assert_eq!(expected, empty);
    }
}
