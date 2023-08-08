use std::{collections::BTreeSet, marker::PhantomData};

use hloo_core::Distance;

use crate::DynBitPermuter;

use super::{block_locator::BlockLocator, extract_key, Index, IndexStats};

pub struct MemIndex<K, V, M> {
    permuter: DynBitPermuter<K, M>,
    block_locator: BlockLocator,
    current_stats: IndexStats,
    data: Vec<(K, V)>,
    _dummy: PhantomData<M>,
}

impl<K, V, M> MemIndex<K, V, M>
where
    K: Copy,
    M: Copy + Ord,
{
    pub fn new(permuter: DynBitPermuter<K, M>) -> Self {
        Self {
            permuter,
            block_locator: BlockLocator::DoubleBsearch,
            current_stats: IndexStats::default(),
            data: Vec::new(),
            _dummy: PhantomData,
        }
    }
}

impl<K, V, M> Index<K, V, M> for MemIndex<K, V, M>
where
    K: Copy + Distance + Ord,
    V: Copy,
    M: Copy + Ord,
{
    type Error = ();

    fn data(&self) -> &[(K, V)] {
        &self.data
    }

    fn permuter(&self) -> DynBitPermuter<K, M> {
        self.permuter.clone()
    }

    fn block_locator(&self) -> BlockLocator {
        self.block_locator
    }

    fn stats(&self) -> &IndexStats {
        &self.current_stats
    }

    fn refresh(&mut self) {
        self.current_stats = self.compute_stats();
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
    use hloo_core::{BitIndex, BitPermuter, Distance};
    use hloo_macros::make_permutations;

    use super::*;

    make_permutations!(struct_name = "Permutations", f = 32, r = 5, k = 1, w = 32);
    // blocks: 7 7 6 6 6
    // mask width: 32 / 5 ; 2 -> 14

    #[test]
    fn test_mem_index_search_works_for_perm0() {
        let mut index = MemIndex::new(Permutations::get_variant(0));
        let data = [
            (Bits::new([0b11111000100010_001000100010001000u32]), 0),
            (Bits::new([0b11111000100010_001000100011111000u32]), 2),
            (Bits::new([0b11001000111110_001000100010001010u32]), 3),
            (Bits::new([0b10011110100010_001000100010001100u32]), 4),
        ];
        index.insert(&data).unwrap();
        let result = index.get_candidates(&data[2].0).block;
        assert_eq!(result, &data[2..3]);
    }
}
