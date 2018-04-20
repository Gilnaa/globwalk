# GlobWalk #

[![Build Status](https://travis-ci.org/Gilnaa/globwalk.svg?branch=master)](https://travis-ci.org/Gilnaa/globwalk)
[![Build status](https://ci.appveyor.com/api/projects/status/81rkf5lcyt1ouh9n/branch/master?svg=true)](https://ci.appveyor.com/project/Gilnaa/globwalk)
[![](https://docs.rs/globwalk/badge.svg)](https://docs.rs/globwalk/)

Recursively find files in a directory using globs.

Based on both `walkdir` & `ignore` (❤), this crate inherits many goodies from
both, such as limiting search depth and amount of open file descriptors.

Licensed under MIT.

### Why not `glob` ###

 - The `glob` crate does not support having `{a,b}` in patterns.
 - `globwalk` can match several glob-patterns at the same time.
 - `globwalk` supports excluding results with `!`.
 - `glob` searches for files in the current working directory, whereas `globwalk` starts at a specified base-dir.

### Documentation ###

[docs.rs/globwalk](https://docs.rs/globwalk/)

### Usage ###

To use this crate, add `globwalk` as a dependency to your project's `Cargo.toml`:

```toml
[dependencies]
globwalk = "0.1"
```

### Example ###

The following piece of code recursively find all mp3 and FLAC files:

```rust,no_run
extern crate globwalk;

use std::fs;

for img in globwalk::glob("*.{png,jpg,gif}").unwrap() {
    if let Ok(img) = img {
        fs::remove_file(img.path()).unwrap();
    }
}
```


### Example: Tweak walk options ###

```rust,no_run
extern crate globwalk;

use std::fs;

let walker = globwalk::glob("*.{png,jpg,gif}")
    .unwrap()
    .max_depth(4)
    .follow_links(true)
    .into_iter()
    .filter_map(Result::ok);
for img in walker {
    fs::remove_file(img.path()).unwrap();
}
```

### Example: Advanced Globbing ###

By using one of the constructors of `globwalk::GlobWalker`, it is possible to alter the base-directory or add multiple patterns.

```rust,no_run
extern crate globwalk;

use std::fs;

let walker = globwalk::GlobWalker::from_patterns(BASE_DIR, &["*.{png,jpg,gif}", "!Pictures/*"])
    .unwrap()
    .into_iter()
    .filter_map(Result::ok);
    
for img in walker {
    fs::remove_file(img.path()).unwrap();
}
```
