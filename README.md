# hloo

`hloo` (Hash LOOkup) is a library for storing hashes and looking them up using distance queries.

`hloo` implements the algorithm described in [this paper by Google](https://static.googleusercontent.com/media/research.google.com/en//pubs/archive/33026.pdf).

`hloo` can perform exceptionally well, achieving roughly O(1) time to search on perfectly uniformly distributed hash data. However, in the real world data is often skewed, and in the worst cases `hloo` can perform as bad as a naive full scan of data. It is always advised to check quality of your hashes!
