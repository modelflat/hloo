pub mod memmap_index;
pub mod memory_index;

use std::hash::Hash;

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

pub trait BitPermuter<K, V, M> {
    /// Apply permutation to bit sequence `key`.
    fn apply(&self, key: K) -> K;

    /// Apply mask to bit sequence `key`.
    fn mask(&self, key: &K) -> M;

    /// Compute distance between `key1` and `key2`.
    fn dist(&self, key1: &K, key2: &K) -> u32;
}

pub trait Index<K, V, M, P>
where
    K: Ord,
    P: BitPermuter<K, V, M>,
{
    type Error;

    /// Load this index's data from a file.
    fn load(&mut self) -> Result<(), Self::Error>
    where
        Self: Sized;

    /// Persist index to a file
    fn save(&self) -> Result<(), Self::Error>
    where
        Self: Sized;

    /// Insert an item into this index.
    fn insert(&mut self, items: &[(K, V)]) -> Result<(), Self::Error>;

    /// Search for an item in this index.
    fn search(&self, key: K, distance: u32) -> Result<Vec<SearchResultItem<V>>, Self::Error>;
}

pub(crate) fn find_block_bounds<K, V, M>(data: &[(K, V)], pos: usize, mask_fn: impl Fn(&K) -> M) -> &[(K, V)]
where
    K: Copy,
    V: Copy,
    M: Ord,
{
    let key = mask_fn(&data[pos].0);
    let mut start = pos;
    while start > 0 {
        start -= 1;
        if key != mask_fn(&data[start].0) {
            start += 1;
            break;
        }
    }
    let mut end = pos;
    while end < &data.len() - 1 {
        end += 1;
        if key != mask_fn(&data[end].0) {
            break;
        }
    }
    &data[start..end]
}

pub(crate) fn filter_by_distance<K, V>(
    data: &[(K, V)],
    key: &K,
    distance_threshold: u32,
    distance_fn: impl Fn(&K, &K) -> u32,
) -> Vec<SearchResultItem<V>>
where
    V: Copy,
{
    data.iter()
        .filter_map(move |(this_key, value)| {
            let dist = distance_fn(this_key, key);
            if dist <= distance_threshold {
                Some(SearchResultItem::new(*value, dist))
            } else {
                None
            }
        })
        .collect()
}
