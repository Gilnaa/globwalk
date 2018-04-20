# GlobWalk #
Recursively find files in a directory using globs.

Based on both `walkdir` &️ `globset` (❤), this crate inherits many goodies from
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


### Example: Tweak walk options

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
