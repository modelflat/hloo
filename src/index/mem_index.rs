use std::{collections::BTreeSet, fmt::Debug, marker::PhantomData};

use bit_permute::Distance;
use thiserror::Error;

use super::{compute_index_stats, extract_key, scan_block, select_block_at, BitPermuter, Index, SearchResultItem};

#[derive(Debug, Error)]
pub enum MemIndexError {
    #[error("distance ({distance}) exceeds maximum allowed distance for index ({max})")]
    DistanceExceedsMax { distance: u32, max: u32 },
}

pub struct MemIndex<K, V, M, P> {
    permuter: P,
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
            data: Vec::new(),
            _dummy: PhantomData,
        }
    }

    pub fn data(&self) -> &[(K, V)] {
        &self.data
    }
}

impl<K, V, M, P> Index<K, V, M, P> for MemIndex<K, V, M, P>
where
    K: Copy + Distance + Ord,
    V: Copy,
    M: Copy + Ord,
    P: BitPermuter<K, M>,
{
    type Error = MemIndexError;

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

    fn search(&self, key: &K, distance: u32) -> Result<Vec<SearchResultItem<V>>, Self::Error> {
        if distance >= self.permuter.n_blocks() {
            return Err(Self::Error::DistanceExceedsMax {
                distance,
                max: self.permuter.n_blocks(),
            });
        }
        let permuted_key = self.permuter.apply(key);
        let masked_key = self.permuter.mask(&permuted_key);
        let location = self
            .data
            .binary_search_by_key(&masked_key, |(key, _)| self.permuter.mask(key));
        match location {
            Ok(pos) => {
                let block = select_block_at(&self.data, pos, |key| self.permuter.mask(key));
                let result = scan_block(block, &permuted_key, distance);
                Ok(result)
            }
            Err(_) => Ok(vec![]),
        }
    }

    fn stats(&self) -> super::IndexStats {
        compute_index_stats(&self.data, |key| self.permuter.mask(key))
    }
}

#[cfg(test)]
mod tests {
    use bit_permute::{BitPermuter, Distance, DynBitPermuter};
    use bit_permute_macro::make_permutations;

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
