mod stats;
pub use stats::IndexStats;

mod mem_index;
pub use mem_index::MemIndex;

mod memmap_index;
pub use memmap_index::{MemMapIndex, MemMapIndexError};

use std::{hash::Hash, path::Path};

use hloo_core::{BitContainer, BitPermuter};

use crate::DynBitPermuter;

use std::cmp::Ordering;

use crate::util::extended_binary_search_by;

/// Locates continuous blocks in sorted slices.
#[derive(Clone, Copy, Debug)]
pub enum BlockLocator {
    /// Performs well on any block size.
    BinarySearch,
}

impl BlockLocator {
    pub fn locate_by<'a, T>(&'_ self, slice: &'a [T], f: impl Fn(&T) -> Ordering) -> &'a [T] {
        match self {
            BlockLocator::BinarySearch => extended_binary_search_by(slice, f),
        }
    }
}

/// Represents a single block of potential candidates for a distance search.
pub struct Candidates<'a, K, V> {
    key: K,
    block: &'a [(K, V)],
}

impl<'a, K, V> Candidates<'a, K, V>
where
    K: BitContainer,
    V: Clone,
{
    pub fn new(key: K, block: &'a [(K, V)]) -> Self {
        Self { key, block }
    }

    /// How many candidates there are.
    pub fn len(&self) -> usize {
        self.block.len()
    }

    pub fn is_empty(&self) -> bool {
        self.block.is_empty()
    }

    /// Performs a full scan of candidates and returns results.
    pub fn scan(&self, distance: u32) -> Vec<SearchResultItem<V>> {
        self.block
            .iter()
            .filter_map(move |(this_key, value)| {
                let dist = this_key.xor_dist(&self.key);
                if dist <= distance {
                    Some(SearchResultItem::new(value.clone(), dist))
                } else {
                    None
                }
            })
            .collect()
    }
}

///
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

/// Search index. Equivalent to notion of "table" in
/// [the paper](https://static.googleusercontent.com/media/research.google.com/en//pubs/archive/33026.pdf)
pub trait Index<K, V, M>
where
    K: BitContainer,
    M: Ord,
    V: Clone,
{
    type Error;

    /// Get permuter reference.
    fn permuter(&self) -> &dyn BitPermuter<K, M>;

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

    /// Retrieve candidates for a given search.
    #[inline(never)]
    fn get_candidates<'a>(&'a self, key: &K) -> Candidates<'a, K, V> {
        let permuter = self.permuter();
        let permuted_key = permuter.apply(key);
        let masked_key = permuter.mask(&permuted_key);
        let block = self
            .block_locator()
            .locate_by(self.data(), |(key, _)| permuter.mask_and_cmp(key, &masked_key));
        Candidates::new(permuted_key, block)
    }

    /// Compute stats for this index.
    fn compute_stats(&self) -> IndexStats {
        let permuter = self.permuter();
        IndexStats::from_data(self.data(), |(key, _)| permuter.mask(key))
    }
}

/// Index that can be persisted to disk or some other storage.
pub trait PersistentIndex<K, M>
where
    Self: Sized,
{
    type Error;

    fn create(permuter: DynBitPermuter<K, M>, sig: u64, path: &Path) -> Result<Self, Self::Error>;

    fn load(permuter: DynBitPermuter<K, M>, sig: u64, path: &Path) -> Result<Self, Self::Error>;

    fn persist(&self) -> Result<(), Self::Error>;
}

/// Extract first element from a tuple.
#[inline(always)]
pub fn extract_key<K: Copy, V>(item: &(K, V)) -> K {
    item.0
}

/// Perform a naive distance search for a key with a given distance.
pub fn naive_search<K: BitContainer, V: Clone>(data: &[(K, V)], key: K, distance: u32) -> Vec<SearchResultItem<V>> {
    Candidates::new(key, data).scan(distance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MyKey(u32);

    impl BitContainer for MyKey {
        type Data = u32;

        fn data(&self) -> &Self::Data {
            unimplemented!()
        }

        fn data_mut(&mut self) -> &mut Self::Data {
            unimplemented!()
        }

        fn bit(&self, _: usize) -> bool {
            unimplemented!()
        }

        fn xor_dist(&self, other: &Self) -> u32 {
            self.0.abs_diff(other.0)
        }
    }

    #[test]
    fn test_candidate_scan_works_correctly() {
        let data = vec![
            (MyKey(1u32), 0),
            (MyKey(2u32), 1),
            (MyKey(2u32), 2),
            (MyKey(3u32), 3),
            (MyKey(4u32), 4),
            (MyKey(4u32), 5),
            (MyKey(4u32), 6),
        ];
        let candidates = Candidates::new(MyKey(1), &data);

        let res = candidates.scan(0);
        assert_eq!(res.len(), 1, "pos 0");
        assert_eq!(res, vec![SearchResultItem::new(0, 0)], "pos 0 - data");
        let res = candidates.scan(1);
        assert_eq!(res.len(), 3, "pos 0-2");
        assert_eq!(
            res,
            vec![
                SearchResultItem::new(0, 0),
                SearchResultItem::new(1, 1),
                SearchResultItem::new(2, 1),
            ],
            "pos 0-2 - data"
        )
    }
}
