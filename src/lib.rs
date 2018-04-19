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
//! A cross platform crate for recursively walking over paths matching a Glob pattern.
//!
//! This crate inherits many features from both `walkdir` and `globset`,
//! like  optionally following symbolic links, limiting of open file descriptors,
//! and more.
//!
//! Glob related options can be set by supplying your own `GlobSet` to `GlobWalk::from_globset`.
//!
//! # Example: Finding image files in the current directory.
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

extern crate walkdir;
extern crate globset;

#[cfg(test)]
extern crate tempdir;

use std::path::{PathBuf, Path};
use std::cmp::Ordering;
use globset::{GlobSetBuilder, Glob, GlobSet};
use walkdir::{WalkDir, DirEntry};

type GlobError = globset::Error;
type WalkError = walkdir::Error;

/// An iterator for recursively yielding glob matches.
///
/// The order of elements yielded by this iterator is unspecified.
pub struct GlobWalker {
    glob: GlobSet,
    base: PathBuf,

    min_depth: usize,
    max_depth: usize,
    follow_links: bool,
    max_open: usize,
    sort_by: Option<Box<
        FnMut(&DirEntry,&DirEntry) -> Ordering + Send + Sync + 'static
    >>,
    contents_first: bool,
}

impl GlobWalker {
    pub fn new<S: AsRef<str>>(pattern: S) -> Result<Self, GlobError> {
        GlobWalker::from_patterns(&[pattern])
    }

    /// Construct a new `GlobWalker` from a list of patterns.
    ///
    /// When iterated, the base directory will be recursively searched for paths
    /// matching `pats`.
    pub fn from_patterns<S: AsRef<str>>(pats: &[S]) -> Result<Self, GlobError> {

        let mut builder = GlobSetBuilder::new();
        for pattern in pats {
            builder.add(Glob::new(pattern.as_ref())?);
        }

        let set = builder.build()?;

        Ok(Self::from_globset(set))
    }

    /// Construct a new `GlobWalker` from a GlobSet
    ///
    /// When iterated, the base directory will be recursively searched for paths
    /// matching `glob`.
    pub fn from_globset(glob: GlobSet) -> Self {
        GlobWalker {
            glob,
            base: ".".into(),
            follow_links: false,
            max_open: 10,
            min_depth: 0,
            max_depth: ::std::usize::MAX,
            sort_by: None,
            contents_first: false,
        }
    }

    /// Change the root dir of the walker.
    pub fn base_dir<P: AsRef<Path>>(mut self, base: P) -> Self {
        self.base = base.as_ref().into();
        self
    }

    /// Set the minimum depth of entries yielded by the iterator.
    ///
    /// The smallest depth is `0` and always corresponds to the path given
    /// to the `new` function on this type. Its direct descendents have depth
    /// `1`, and their descendents have depth `2`, and so on.
    pub fn min_depth(mut self, depth: usize) -> Self {
        self.min_depth = depth;
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
        self.max_depth = depth;
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
        self.follow_links = yes;
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
        self.max_open = n;
        self
    }

// FIXME: See next FIXME
//    /// Set a function for sorting directory entries.
//    ///
//    /// If a compare function is set, the resulting iterator will return all
//    /// paths in sorted order. The compare function will be called to compare
//    /// entries from the same directory.
//    pub fn sort_by<F>(mut self, cmp: F) -> Self
//        where F: FnMut(&DirEntry, &DirEntry) -> Ordering + Send + Sync + 'static
//    {
//        self.sort_by = Some(Box::new(cmp));
//        self
//    }

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
        self.contents_first = yes;
        self
    }
}

impl IntoIterator for GlobWalker {
    type Item = Result<DirEntry, WalkError>;
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        let walker = WalkDir::new(&self.base)
            .min_depth(self.min_depth)
            .max_depth(self.max_depth)
            .max_open(self.max_open)
            .follow_links(self.follow_links)
            .contents_first(self.contents_first);

        // FIXME: This cannot compile right now.
        // let walker = if let Some(sorter) = self.sort_by.take() {
        //     walker.sort_by(move |a, b| sorter(a, b))
        // }
        // else {
        //     walker
        // };

        IntoIter {
            glob: self.glob,
            base: self.base,
            walker: walker.into_iter()
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
    glob: GlobSet,
    base: PathBuf,
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
                    if self.glob.is_match((&e).path().strip_prefix(&*self.base).unwrap()) {
                        return Some(Ok(e));
                    }
                },
                Err(e) => {
                    return Some(Err(e));
                }
            }
        }

        None
    }
}

pub fn glob<S: AsRef<str>>(pattern: S) -> Result<GlobWalker, GlobError> {
    GlobWalker::new(pattern)
}


#[cfg(test)]
mod tests {
    use super::*;
    use ::tempdir::TempDir;
    use ::std::fs::{File, create_dir_all};

    fn touch(dir: &TempDir, names: &[&str]) {
        for name in names {
            let name = normalize_path_sep(name);
            File::create(dir.path().join(name)).expect("Failed to create a test file");
        }
    }

    fn normalize_path_sep<S: AsRef<str>>(s: S) -> String {
        s.as_ref().replace("[/]", if cfg!(windows) {"\\"} else {"/"})
    }

    #[test]
    fn do_the_globwalk() {
        let dir = TempDir::new("globset_walkdir").expect("Failed to create temporary folder");
        let dir_path = dir.path();
        create_dir_all(dir_path.join("src/some_mod")).expect("");
        create_dir_all(dir_path.join("tests")).expect("");
        create_dir_all(dir_path.join("contrib")).expect("");

        touch(&dir, &[
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
        ][..]);


        let mut builder = GlobSetBuilder::new();
        builder.add(Glob::new("src/**/*.rs").unwrap());
        builder.add(Glob::new("*.c").unwrap());
        builder.add(Glob::new("**/lib.rs").unwrap());
        builder.add(Glob::new("**/*.{md,rst}").unwrap());
        let set = builder.build().unwrap();

        let mut expected: Vec<_> = ["src[/]some_mod[/]unexpected.rs",
                                    "src[/]world.rs",
                                    "src[/]hello.rs",
                                    "lib.c",
                                    "contrib[/]lib.rs",
                                    "contrib[/]README.md",
                                    "contrib[/]README.rst"].iter().map(normalize_path_sep).collect();

        for matched_file in GlobWalker::from_globset(set)
                                        .base_dir(dir_path)
                                        .into_iter()
                                        .filter_map(Result::ok) {
            let path = matched_file.path().strip_prefix(dir_path).unwrap().to_str().unwrap();
            let path = path.replace("[/]", if cfg!(windows) {"\\"} else {"/"});

            println!("path = {}", path);

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
    fn find_image_files() {
        let dir = TempDir::new("globset_walkdir").expect("Failed to create temporary folder");
        let dir_path = dir.path();

        touch(&dir, &[
            "a.rs",
            "a.jpg",
            "a.png",
            "b.docx",
        ][..]);


        let mut expected = vec!["a.jpg", "a.png"];

        for matched_file in GlobWalker::new("*.{png,jpg,gif}")
                                        .unwrap()
                                        .base_dir(dir_path)
                                        .into_iter()
                                        .filter_map(Result::ok) {
            let path = matched_file.path().strip_prefix(dir_path).unwrap().to_str().unwrap();
            let path = path.replace("[/]", if cfg!(windows) {"\\"} else {"/"});

            println!("path = {}", path);

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
