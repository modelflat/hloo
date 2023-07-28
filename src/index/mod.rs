mod mem_index;
pub use mem_index::{MemIndex, MemIndexError};

#[cfg(feature = "memmap_index")]
mod memmap_index;
#[cfg(feature = "memmap_index")]
pub use memmap_index::{MemMapIndex, MemMapIndexError};

use std::{hash::Hash, path::Path};

use bit_permute::{BitPermuter, Distance};

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
    K: Ord,
    P: BitPermuter<K, M>,
{
    type Error;

    /// Insert items into this index.
    fn insert(&mut self, items: &[(K, V)]) -> Result<(), Self::Error>;

    /// Remove items from this index.
    fn remove(&mut self, keys: &[K]) -> Result<(), Self::Error>;

    /// Search for an item in this index.
    fn search(&self, key: &K, distance: u32) -> Result<Vec<SearchResultItem<V>>, Self::Error>;

    /// Compute stats for this index.
    fn stats(&self) -> IndexStats;
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

#[derive(Default)]
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

pub fn select_block_at<K, V, M>(data: &[(K, V)], pos: usize, mask_fn: impl Fn(&K) -> M) -> &[(K, V)]
where
    K: Copy,
    V: Copy,
    M: Ord,
{
    let key = mask_fn(&data[pos].0);
    let mut start = pos;
    while start > 0 && key == mask_fn(&data[start - 1].0) {
        start -= 1;
    }
    let mut end = pos;
    while end < data.len() - 1 && key == mask_fn(&data[end + 1].0) {
        end += 1;
    }
    &data[start..=end]
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
    use bit_permute::Distance;

    use crate::index::{scan_block, SearchResultItem};

    use super::{compute_index_stats, select_block_at};

    fn id<T: Copy + Ord>(x: &T) -> T {
        *x
    }

    struct MyKey(u32);

    impl Distance for MyKey {
        fn xor_dist(&self, other: &Self) -> u32 {
            self.0.abs_diff(other.0)
        }
    }

    #[test]
    fn test_scan_block_works_correctly() {
        let data = vec![
            (MyKey(1u32), 0),
            (MyKey(2u32), 1),
            (MyKey(2u32), 2),
            (MyKey(3u32), 3),
            (MyKey(4u32), 4),
            (MyKey(4u32), 5),
            (MyKey(4u32), 6),
        ];

        let res = scan_block(&data, &MyKey(1), 0);
        assert_eq!(res.len(), 1, "pos 0");
        assert_eq!(res, vec![SearchResultItem::new(0, 0)], "pos 0 - data");
        let res = scan_block(&data, &MyKey(1), 1);
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

    #[test]
    fn test_select_block_at_works_correctly() {
        let data = vec![
            (1u32, 0),
            (2u32, 1),
            (2u32, 2),
            (3u32, 3),
            (4u32, 4),
            (4u32, 5),
            (4u32, 6),
        ];

        let result0 = select_block_at(&data, 0, id);
        assert_eq!(result0.len(), 1, "pos 0");
        assert_eq!(result0, &data[0..1], "pos 0 - data");
        let result0 = select_block_at(&data, 1, id);
        assert_eq!(result0.len(), 2, "pos 1");
        assert_eq!(result0, &data[1..3], "pos 1 - data");
        let result0 = select_block_at(&data, 2, id);
        assert_eq!(result0.len(), 2, "pos 2");
        assert_eq!(result0, &data[1..3], "pos 2 - data");
        let result0 = select_block_at(&data, 3, id);
        assert_eq!(result0.len(), 1, "pos 3");
        assert_eq!(result0, &data[3..4], "pos 3 - data");
        let result0 = select_block_at(&data, 4, id);
        assert_eq!(result0.len(), 3, "pos 4");
        assert_eq!(result0, &data[4..7], "pos 4 - data");
        let result0 = select_block_at(&data, 5, id);
        assert_eq!(result0.len(), 3, "pos 5");
        assert_eq!(result0, &data[4..7], "pos 5 - data");
        let result0 = select_block_at(&data, 6, id);
        assert_eq!(result0.len(), 3, "pos 6");
        assert_eq!(result0, &data[4..7], "pos 6 - data");
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
