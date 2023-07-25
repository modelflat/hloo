use std::marker::PhantomData;

use crate::util::merge_sorted_alloc;

use super::{filter_by_distance, find_block_bounds, BitPermuter, Index, SearchResultItem};

pub struct MemoryIndex<K, V, M, P> {
    permuter: P,
    data: Vec<(K, V)>,
    mask_type: PhantomData<M>,
}

impl<K, V, M, P> MemoryIndex<K, V, M, P>
where
    K: Copy,
    M: Copy + Ord,
    P: BitPermuter<K, M>,
{
    pub fn new(permuter: P) -> Self {
        Self {
            permuter,
            data: Default::default(),
            mask_type: Default::default(),
        }
    }
}

impl<K, V, M, P> Index<K, V, M, P> for MemoryIndex<K, V, M, P>
where
    K: Copy + Ord,
    V: Copy,
    M: Copy + Ord,
    P: BitPermuter<K, M>,
{
    type Error = ();

    fn load(&mut self) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        unimplemented!("Memory index cannot be loaded from a file!");
    }

    fn save(&self) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        unimplemented!("Memory index cannot be saved to a file!");
    }

    fn insert(&mut self, items: &[(K, V)]) -> Result<(), Self::Error> {
        // TODO this is inefficient
        let mut permuted: Vec<_> = items.iter().map(|(k, v)| (self.permuter.apply(*k), *v)).collect();
        permuted.sort_by_key(|(k, _)| *k);
        let new = merge_sorted_alloc(&self.data, &permuted, |(k, _)| k);
        self.data = new;
        Ok(())
    }

    fn search(&self, key: K, distance: u32) -> Result<Vec<SearchResultItem<V>>, Self::Error> {
        let permuted_key = self.permuter.apply(key);
        let masked_key = self.permuter.mask(&permuted_key);
        let block = self
            .data
            .binary_search_by_key(&masked_key, |(key, _)| self.permuter.mask(key))
            .map_or_else(
                |_| &self.data[0..0],
                |pos| find_block_bounds(&self.data, pos, |key| self.permuter.mask(key)),
            );
        let result = filter_by_distance(block, &permuted_key, distance, |k1, k2| self.permuter.dist(k1, k2));
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bit_permute_macro::make_permutations;

    use super::*;

    make_permutations!(struct_name = "Permutation", f = 32, r = 5, k = 2, w = 32);
    // blocks: 7 7 6 6 6
    // mask width: 32 / 5 ; 2 -> 14

    pub struct MyPermuter(Arc<dyn Permutation>);

    impl BitPermuter<Bits, Mask> for MyPermuter {
        fn apply(&self, key: Bits) -> Bits {
            self.0.apply(key)
        }

        fn mask(&self, key: &Bits) -> Mask {
            self.0.mask(&key)
        }

        fn dist(&self, key1: &Bits, key2: &Bits) -> u32 {
            key1.xor_count_ones(key2)
        }
    }

    #[test]
    fn test_memory_index_search_works() {
        let mut index = MemoryIndex::new(MyPermuter(PermutationUtil::get_variant(0)));
        index
            .insert(&[
                (Bits::new([0b11111000100010_001000100010001000u32]), 0),
                (Bits::new([0b11111000100010_001000100011111000u32]), 2),
                (Bits::new([0b11001000111110_001000100010001010u32]), 3),
                (Bits::new([0b10011110100010_001000100010001100u32]), 4),
            ])
            .unwrap();
        let result = index
            .search(Bits::new([0b11001000111110_001011100010001010u32]), 2)
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], SearchResultItem::new(3, 2))
    }
}
