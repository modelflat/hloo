# hloo

`hloo` (Hash LOOkup) is a library for storing hashes and looking them up using distance queries.

`hloo` implements the algorithm described in [this paper by Google](https://static.googleusercontent.com/media/research.google.com/en//pubs/archive/33026.pdf).

Below are the results from benchmarking `hloo` against the naive full-scan. Results are obtained on 2021 MacBook Pro M1. The dataset size is 1M of uniformely distributed hashes (generated randomly). See `benches/all.rs` for more info.

|lookup method|avg time per query|
|-|-|-|
|naive|1.0988 ms|
|hloo (in-memory index)|543.36 ns|
|hloo (memory-mapped index)|553.40 ns|
