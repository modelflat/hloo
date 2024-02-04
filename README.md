# hloo

`hloo` (Hash LOOkup) is a library for storing hashes and finding them using distance queries. It uses an algorithm described in [this paper by Google](https://static.googleusercontent.com/media/research.google.com/en//pubs/archive/33026.pdf) (without the compression part).

This algorithm can perform exceptionally well, achieving roughly O(1) time to search on perfectly uniformly distributed hash data. However, in the real world data is often skewed, and in the worst cases (i.e. huge chunks of hashes are the same value) it will perform on par or slightly worse than a naive full scan of data. Always check the quality of your hashes!
