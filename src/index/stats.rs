/// Statistics of the index.
#[derive(Default, Debug)]
pub struct IndexStats {
    pub n_items: usize,
    pub n_blocks: usize,
    pub min_block_size: usize,
    pub avg_block_size: usize,
    pub max_block_size: usize,
}

impl IndexStats {
    pub fn from_data<K, V, M>(data: &[(K, V)], mask_fn: impl Fn(&K) -> M) -> Self
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
                n_blocks,
                n_items: data.len(),
                min_block_size: min.min(curr_size),
                avg_block_size: data.len() / n_blocks,
                max_block_size: max.max(curr_size),
            }
        } else {
            IndexStats::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let stats = IndexStats::from_data(&data, id);
        assert_eq!(stats.n_blocks, 4, "n blocks");
        assert_eq!(stats.n_items, data.len(), "n items");
        assert_eq!(stats.min_block_size, 1, "min");
        assert_eq!(stats.avg_block_size, 2, "avg");
        assert_eq!(stats.max_block_size, 3, "max");
    }
}
