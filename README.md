# GlobWalk #
A cross platform crate for recursively walking over paths matching a Glob pattern.

Based on both `walkdir` &️ `globset` (❤), this crate inherits many goodies from both, such as limiting search depth and amount of open file descriptors. 

Licensed under MIT.

### Why not `glob` ###

 - The `glob` crate does not support having `{a,b}` in patterns.
 - `globwalk` can match several glob-patterns at the same time.
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

```rust
extern crate globwalk;
use globwalk::GlobWalker;

fn search_and_destroy() {
    for track in GlobWalker::from_patterns(&["**/*.{mp3,flac}"], ".") {
        if let Ok(track) = track {
            // Destroy satanic rhythms
            std::fs::remove_file(track.path());
        } 
    }
}
```


### Example: Tweak walk options

```rust
extern crate globwalk;
use globwalk::GlobWalker;

fn search_and_destroy() {
    let walker = GlobWalker::from_patterns(&["**/*.{mp3,flac}"], ".")
                    .max_depth(4)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(Result::ok);
                    
    for track in walker {
        // Destroy symbolic satanic rhythms, but do not stray far.
        std::fs::remove_file(track.path()); 
    }
}
```