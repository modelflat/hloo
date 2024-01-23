# hloo

`hloo` (Hash LOOkup) is a library for storing hashes and looking them up using distance queries.

`hloo` implements the algorithm described in [this paper by Google](https://static.googleusercontent.com/media/research.google.com/en//pubs/archive/33026.pdf).

`hloo` can perform exceptionally well, achieving roughly O(1) time to search on perfectly uniformly distributed hash data. However, in the real world data is often skewed, and in the worst cases (i.e. huge chunks of hashes are the same value) `hloo` will perform on par or slightly worse than a naive full scan of data. Always check the quality of your hashes!
