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

extern crate walkdir;
extern crate globset;

#[cfg(test)]
extern crate tempdir;

use globset::{GlobSet, Glob, GlobSetBuilder};
use std::path::{PathBuf, Path};
use std::fs::{File, create_dir_all};

/// An iterator for recursively yielding glob matches.
///
/// The order of elements yielded by this iterator is unspecified.
pub struct GlobWalker<'a> {
    glob: &'a GlobSet,
    base: PathBuf,
    walker: walkdir::IntoIter,
}

impl<'a> GlobWalker<'a> {
    pub fn new<P: AsRef<Path>>(glob: &'a GlobSet, base: P) -> Self {
        GlobWalker {
            glob,
            base: base.as_ref().into(),
            walker: walkdir::WalkDir::new(base).into_iter(),
        }
    }
}

impl<'a> Iterator for GlobWalker<'a> {
    type Item = walkdir::DirEntry;

    // Possible optimization - Do not descend into directory that will never be a match
    fn next(&mut self) -> Option<Self::Item> {
        for entry in &mut self.walker {
            if let Ok(entry) = entry {
                // Strip the common base directory so that the matcher will be
                // able to recognize the file name.
                // `unwrap` here is safe, since walkdir returns the files with relation
                // to the given base-dir.
                if self.glob.is_match(entry.path().strip_prefix(&*self.base).unwrap()) {
                    return Some(entry)
                }
            }
        }

        None
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use ::tempdir::TempDir;

    fn touch(dir: &TempDir, names: &[&str]) {
        for name in names {
            File::create(dir.path().join(name)).expect("Failed to create a test file");
        }
    }

    // FIXME: This test doesn't work on Windows because of the path separators.
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
            "src/hello.rs",
            "src/world.rs",
            "src/some_mod/unexpected.rs",
            "src/cruel.txt",
            "contrib/README.md",
            "contrib/README.rst",
            "contrib/lib.rs",
        ][..]);


        let mut builder = GlobSetBuilder::new();
        builder.add(Glob::new("src/**/*.rs").unwrap());
        builder.add(Glob::new("*.c").unwrap());
        builder.add(Glob::new("**/lib.rs").unwrap());
        builder.add(Glob::new("**/*.{md,rst}").unwrap());
        let set = builder.build().unwrap();

        let mut expected = vec!["src/some_mod/unexpected.rs",
                                "src/world.rs",
                                "src/hello.rs",
                                "lib.c",
                                "contrib/lib.rs",
                                "contrib/README.md",
                                "contrib/README.rst"];

        for matched_file in GlobWalker::new(&set, dir_path) {
            let path = matched_file.path().strip_prefix(dir_path).unwrap().to_str().unwrap();

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
