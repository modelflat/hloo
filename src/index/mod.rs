mod block_locator;
pub use block_locator::BlockLocator;

mod mem_index;
pub use mem_index::MemIndex;

mod memmap_index;
pub use memmap_index::{MemMapIndex, MemMapIndexError};

use std::{hash::Hash, path::Path};

use bit_permute::{BitPermuter, Distance};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SearchError<T> {
    #[error("distance ({distance}) exceeds maximum allowed distance for index ({max})")]
    DistanceExceedsMax { distance: u32, max: u32 },
    #[error("index error: {0}")]
    IndexError(#[from] T),
}

#[derive(Clone, Copy, Eq, Debug)]
pub struct SearchResultItem<V> {
    data: V,
    distance: u32,
}

impl<V> SearchResultItem<V> {
    pub fn new(data: V, distance: u32) -> Self {
        Self { data, distance }
    }

    pub fn data(&self) -> &V {
        &self.data
    }

    pub fn distance(&self) -> u32 {
        self.distance
    }
}

impl<V> PartialEq for SearchResultItem<V>
where
    V: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<V> Hash for SearchResultItem<V>
where
    V: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

pub trait Index<K, V, M, P>
where
    K: Distance,
    M: Ord,
    V: Clone,
    P: BitPermuter<K, M>,
{
    type Error;

    /// Get permuter reference.
    fn permuter(&self) -> &P;

    /// Get currently used BlockLocator.
    fn block_locator(&self) -> BlockLocator;

    /// Get data as a slice.
    fn data(&self) -> &[(K, V)];

    /// Get stats for this index.
    fn stats(&self) -> &IndexStats;

    /// Refresh index: recompute stats etc.
    fn refresh(&mut self);

    /// Insert items into this index.
    fn insert(&mut self, items: &[(K, V)]) -> Result<(), Self::Error>;

    /// Remove items from this index.
    fn remove(&mut self, keys: &[K]) -> Result<(), Self::Error>;

    /// Search for an item in this index.
    fn search(&self, key: &K, distance: u32) -> Result<Vec<SearchResultItem<V>>, SearchError<Self::Error>> {
        let permuter = self.permuter();
        if distance >= permuter.n_blocks() {
            return Err(SearchError::DistanceExceedsMax {
                distance,
                max: permuter.n_blocks(),
            });
        }
        let permuted_key = permuter.apply(key);
        let block = self
            .block_locator()
            .locate(self.data(), &permuted_key, |key| permuter.mask(key));
        Ok(scan_block(block, &permuted_key, distance))
    }
}

pub trait PersistentIndex<P>
where
    Self: Sized,
{
    type Error;

    fn create(permuter: P, sig: u64, path: &Path) -> Result<Self, Self::Error>;

    fn load(permuter: P, sig: u64, path: &Path) -> Result<Self, Self::Error>;

    fn persist(&self) -> Result<(), Self::Error>;
}

/// Extract first element from a tuple.
#[inline(always)]
pub fn extract_key<K: Copy, V>(item: &(K, V)) -> K {
    item.0
}

#[derive(Default, Debug)]
pub struct IndexStats {
    pub min_block_size: usize,
    pub avg_block_size: usize,
    pub max_block_size: usize,
    pub n_blocks: usize,
}

pub fn compute_index_stats<K, V, M>(data: &[(K, V)], mask_fn: impl Fn(&K) -> M) -> IndexStats
where
    K: Copy,
    V: Copy,
    M: Ord,
{
    let mut it = data.iter().map(|(k, _)| mask_fn(k));
    if let Some(mut prev_key) = it.next() {
        let mut curr_size = 1usize;
        let mut n_blocks = 1usize;
        let mut min = usize::MAX;
        let mut max = 0;
        for key in it {
            if prev_key == key {
                curr_size += 1;
            } else {
                min = min.min(curr_size);
                max = max.max(curr_size);
                prev_key = key;
                n_blocks += 1;
                curr_size = 1;
            }
        }
        IndexStats {
            min_block_size: min.min(curr_size),
            avg_block_size: data.len() / n_blocks,
            max_block_size: max.max(curr_size),
            n_blocks,
        }
    } else {
        IndexStats::default()
    }
}

pub fn scan_block<K, V>(data: &[(K, V)], key: &K, distance_threshold: u32) -> Vec<SearchResultItem<V>>
where
    K: Distance,
    V: Clone,
{
    data.iter()
        .filter_map(move |(this_key, value)| {
            let dist = this_key.xor_dist(key);
            if dist <= distance_threshold {
                Some(SearchResultItem::new(value.clone(), dist))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::compute_index_stats;

    fn id<T: Copy + Ord>(x: &T) -> T {
        *x
    }

    #[test]
    fn test_compute_index_stats_works_correctly() {
        let data = vec![
            (1u32, 0),
            (2u32, 1),
            (2u32, 2),
            (3u32, 3),
            (3u32, 3),
            (4u32, 4),
            (4u32, 5),
            (4u32, 6),
        ];

        let stats = compute_index_stats(&data, id);
        assert_eq!(stats.min_block_size, 1, "min");
        assert_eq!(stats.avg_block_size, 2, "avg");
        assert_eq!(stats.max_block_size, 3, "max");
        assert_eq!(stats.n_blocks, 4, "n");
    }
}
