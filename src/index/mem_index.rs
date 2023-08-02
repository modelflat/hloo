use std::{collections::BTreeSet, marker::PhantomData};

use bit_permute::Distance;

use super::{block_locator::BlockLocator, extract_key, BitPermuter, Index, IndexStats};

pub struct MemIndex<K, V, M, P> {
    permuter: P,
    block_locator: BlockLocator,
    current_stats: IndexStats,
    data: Vec<(K, V)>,
    _dummy: PhantomData<M>,
}

impl<K, V, M, P> MemIndex<K, V, M, P>
where
    K: Copy,
    M: Copy + Ord,
    P: BitPermuter<K, M>,
{
    pub fn new(permuter: P) -> Self {
        Self {
            permuter,
            block_locator: BlockLocator::DoubleBsearch,
            current_stats: IndexStats::default(),
            data: Vec::new(),
            _dummy: PhantomData,
        }
    }

    pub fn update_block_locator(&mut self) {
        // TODO settings for index
        if self.current_stats.max_block_size < 5 {
            self.block_locator = BlockLocator::Naive;
        } else {
            self.block_locator = BlockLocator::DoubleBsearch;
        }
    }
}

impl<K, V, M, P> Index<K, V, M, P> for MemIndex<K, V, M, P>
where
    K: Copy + Distance + Ord,
    V: Copy,
    M: Copy + Ord,
    P: BitPermuter<K, M>,
{
    type Error = ();

    fn data(&self) -> &[(K, V)] {
        &self.data
    }

    fn permuter(&self) -> &P {
        &self.permuter
    }

    fn block_locator(&self) -> BlockLocator {
        self.block_locator
    }

    fn stats(&self) -> &IndexStats {
        &self.current_stats
    }

    fn refresh(&mut self) {
        self.current_stats = IndexStats::from_data(&self.data, |key| self.permuter.mask(key));
        self.update_block_locator();
    }

    fn insert(&mut self, items: &[(K, V)]) -> Result<(), Self::Error> {
        let items_permuted = items.iter().map(|(k, v)| (self.permuter.apply(k), *v));
        self.data.extend(items_permuted);
        self.data.sort_unstable_by_key(extract_key);
        Ok(())
    }

    fn remove(&mut self, keys: &[K]) -> Result<(), Self::Error> {
        let set: BTreeSet<_> = keys.iter().map(|k| self.permuter.apply(k)).collect();
        self.data.retain(|(k, _)| !set.contains(k));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bit_permute::{BitIndex, BitPermuter, Distance, DynBitPermuter};
    use bit_permute_macro::make_permutations;

    use crate::index::SearchResultItem;

    use super::*;

    make_permutations!(struct_name = "Permutations", f = 32, r = 5, k = 2, w = 32);
    // blocks: 7 7 6 6 6
    // mask width: 32 / 5 ; 2 -> 14

    #[test]
    fn test_mem_index_search_works() {
        let mut index = MemIndex::new(Permutations::get_variant(0));
        index
            .insert(&[
                (Bits::new([0b11111000100010_001000100010001000u32]), 0),
                (Bits::new([0b11111000100010_001000100011111000u32]), 2),
                (Bits::new([0b11001000111110_001000100010001010u32]), 3),
                (Bits::new([0b10011110100010_001000100010001100u32]), 4),
            ])
            .unwrap();
        let result = index
            .search(&Bits::new([0b11001000111110_001011100010001010u32]), 2)
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], SearchResultItem::new(3, 2))
    }
}
