# hloo

`hloo` (Hash LOOkup) is a library for storing hashes and looking them up using distance queries.

`hloo` implements an algorithm described in [this paper](https://static.googleusercontent.com/media/research.google.com/en//pubs/archive/33026.pdf) by Google.

Below are the results from benchmarking `hloo` against the naive full-scan. Results are obtained on 2021 MacBook Pro M1. The dataset size is 10M uniformely distributed hashes (generated randomly). See `benches/all.rs` for more info.

|lookup method|avg time per query|speedup|
|-|-|-|
|naive|10.664 ms|-|
|hloo (in-memory index)|635 ns|x16794|
|hloo (memory-mapped index)|641 ns|x16636|
