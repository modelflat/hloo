use std::{io, marker::PhantomData, path::PathBuf};

use crate::mmvec::MmVec;

use super::{compute_index_stats, scan_block, select_block_at, BitPermuter, Index, SearchResultItem};

pub struct MemMapIndex<K, V, M, P> {
    permuter: P,
    path: PathBuf,
    data: MmVec<(K, V)>,
    mask_type: PhantomData<M>,
}

impl<K, V, M, P> MemMapIndex<K, V, M, P>
where
    K: Copy,
    V: Copy,
    M: Copy + Ord,
    P: BitPermuter<K, M>,
{
    pub fn new(permuter: P, path: PathBuf) -> Result<Self, io::Error> {
        Ok(Self {
            permuter,
            path: path.clone(),
            data: MmVec::with_length_uninit(0, path)?,
            mask_type: Default::default(),
        })
    }

    fn as_slice(&self) -> &[(K, V)] {
        unsafe { self.data.as_slice() }
    }
}

impl<K, V, M, P> Index<K, V, M, P> for MemMapIndex<K, V, M, P>
where
    K: Copy + Ord,
    V: Copy,
    M: Copy + Ord,
    P: BitPermuter<K, M>,
{
    type Error = io::Error;

    fn load(&mut self) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        self.data = MmVec::from_path(self.path.clone())?;
        Ok(())
    }

    fn save(&self) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        self.data.flush()
    }

    fn insert(&mut self, items: &[(K, V)]) -> Result<(), Self::Error> {
        let mut permuted: Vec<_> = items.iter().map(|(k, v)| (self.permuter.apply(*k), *v)).collect();
        permuted.sort_by_key(|(k, _)| *k);
        self.data.merge_sorted(&permuted, |(k, _)| k)?;
        Ok(())
    }

    fn search(&self, key: K, distance: u32) -> Result<Vec<SearchResultItem<V>>, Self::Error> {
        let data = self.as_slice();
        let permuted_key = self.permuter.apply(key);
        let masked_key = self.permuter.mask(&permuted_key);
        let block = self
            .data
            .binary_search_by_key(&masked_key, |(key, _)| self.permuter.mask(key))
            .map_or_else(
                |_| &data[0..0],
                |pos| select_block_at(data, pos, |key| self.permuter.mask(key)),
            );
        let result = scan_block(block, &permuted_key, distance, |k1, k2| self.permuter.dist(k1, k2));
        Ok(result)
    }

    fn stats(&self) -> super::IndexStats {
        compute_index_stats(self.as_slice(), |key| self.permuter.mask(key))
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

    struct MyPermuter(Arc<dyn Permutation>);

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
    fn test_memvec_index_search_works() {
        let tempdir = tempfile::tempdir().expect("failed to create temp dir");
        let mut index = MemMapIndex::new(
            MyPermuter(PermutationUtil::get_variant(0)),
            tempdir.path().join("storage.bin"),
        )
        .expect("failed to create memory-mapped vector");
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
