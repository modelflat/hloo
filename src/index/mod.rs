mod block_locator;
pub use block_locator::BlockLocator;

mod stats;
pub use stats::IndexStats;

mod mem_index;
pub use mem_index::MemIndex;

mod memmap_index;
pub use memmap_index::{MemMapIndex, MemMapIndexError};

use std::{hash::Hash, path::Path};

use bit_permute::{BitPermuter, Distance};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("distance ({distance}) exceeds maximum allowed distance for index ({max})")]
    DistanceExceedsMax { distance: u32, max: u32 },
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
    fn search(&self, key: &K, distance: u32) -> Result<Vec<SearchResultItem<V>>, SearchError> {
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
