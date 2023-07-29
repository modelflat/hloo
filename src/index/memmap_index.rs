use std::{
    collections::BTreeSet,
    marker::PhantomData,
    path::{Path, PathBuf},
};

use bit_permute::Distance;
use thiserror::Error;

use crate::mmvec::{MmVec, MmVecError};

use super::{
    compute_index_stats, extract_key, scan_block, select_block_at, BitPermuter, Index, PersistentIndex,
    SearchResultItem,
};

#[derive(Debug, Error)]
pub enum MemMapIndexError {
    #[error("distance ({distance}) exceeds maximum allowed distance for index ({max})")]
    DistanceExceedsMax { distance: u32, max: u32 },
    #[error("mmvec error: {0}")]
    IoError(#[from] MmVecError),
}

pub struct MemMapIndex<K, V, M, P>
where
    (K, V): Copy,
{
    permuter: P,
    data: MmVec<(K, V)>,
    _dummy: PhantomData<M>,
}

impl<K, V, M, P> MemMapIndex<K, V, M, P>
where
    (K, V): Copy,
{
    pub fn new(permuter: P, sig: u64, path: PathBuf) -> Result<Self, MemMapIndexError> {
        Ok(Self {
            permuter,
            data: MmVec::new_empty(sig, path)?,
            _dummy: PhantomData,
        })
    }

    pub fn data(&self) -> &[(K, V)] {
        unsafe { self.data.as_slice() }
    }

    pub fn destroy(self) -> Result<(), MemMapIndexError> {
        self.data.destroy()?;
        Ok(())
    }
}

impl<K, V, M, P> Index<K, V, M, P> for MemMapIndex<K, V, M, P>
where
    K: Copy + Distance + Ord,
    V: Copy,
    M: Copy + Ord,
    P: BitPermuter<K, M>,
{
    type Error = MemMapIndexError;

    fn insert(&mut self, items: &[(K, V)]) -> Result<(), Self::Error> {
        let mut permuted: Vec<_> = items.iter().map(|(k, v)| (self.permuter.apply(k), *v)).collect();
        // pre-sort the permuted items to create a "two-sorted-sequences" pattern
        permuted.sort_unstable_by_key(extract_key);
        self.data.insert_sorted(&permuted, extract_key)?;
        Ok(())
    }

    fn remove(&mut self, keys: &[K]) -> Result<(), Self::Error> {
        let set: BTreeSet<_> = keys.iter().map(|k| self.permuter.apply(k)).collect();
        self.data.remove_matching(|(k, _)| set.contains(k), extract_key)?;
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
            .data()
            .binary_search_by_key(&masked_key, |(key, _)| self.permuter.mask(key));
        match location {
            Ok(pos) => {
                let block = select_block_at(self.data(), pos, |key| self.permuter.mask(key));
                let result = scan_block(block, &permuted_key, distance);
                Ok(result)
            }
            Err(_) => Ok(vec![]),
        }
    }

    fn stats(&self) -> super::IndexStats {
        compute_index_stats(self.data(), |key| self.permuter.mask(key))
    }
}

impl<K, V, M, P> PersistentIndex<P> for MemMapIndex<K, V, M, P>
where
    (K, V): Copy,
{
    type Error = MemMapIndexError;

    fn create(permuter: P, sig: u64, path: &Path) -> Result<Self, Self::Error> {
        let data = MmVec::new_empty(sig, path.to_path_buf())?;
        Ok(Self {
            permuter,
            data,
            _dummy: PhantomData,
        })
    }

    fn load(permuter: P, sig: u64, path: &Path) -> Result<Self, Self::Error> {
        let data = MmVec::from_path(sig, path.to_path_buf())?;
        Ok(Self {
            permuter,
            data,
            _dummy: PhantomData,
        })
    }

    fn persist(&self) -> Result<(), Self::Error> {
        self.data.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bit_permute::{BitPermuter, Distance, DynBitPermuter};
    use bit_permute_macro::make_permutations;

    use super::*;

    make_permutations!(struct_name = "Permutations", f = 32, r = 5, k = 1, w = 32);
    // blocks: 7 7 6 6 6
    // mask width: 32 / 5 ; 2 -> 14

    #[test]
    fn memmap_index_search_works_correctly() {
        let tempdir = tempfile::tempdir().expect("failed to create temp dir");
        for (i, perm) in Permutations::get_all_variants().into_iter().enumerate() {
            let index_path = tempdir.path().join("storage.bin");
            let mut index =
                MemMapIndex::new(perm, 0, index_path.clone()).expect("failed to create memory-mapped vector");
            index
                .insert(&[
                    (Bits::new([0b11111000100010_001000100010001000u32]), 0),
                    (Bits::new([0b11111000100010_001000100011111000u32]), 2),
                    (Bits::new([0b11001000111110_001000100010000000u32]), 3),
                    (Bits::new([0b10011110100010_001000100010001100u32]), 4),
                ])
                .unwrap();
            // NOTE: only distance 0 search can be tested here
            let result = index
                .search(&Bits::new([0b11001000111110_001000100010000000u32]), 0)
                .unwrap();
            assert_eq!(result.len(), 1, "[{}] number of search results is wrong", i);
            assert_eq!(result[0], SearchResultItem::new(3, 2), "[{}] search result is wrong", i);
        }
    }

    #[test]
    fn memmap_index_insert_works_correctly() {
        let tempdir = tempfile::tempdir().expect("failed to create temp dir");
        for (i, perm) in Permutations::get_all_variants().into_iter().enumerate() {
            let index_path = tempdir.path().join("storage.bin");
            let mut index = MemMapIndex::new(perm.clone(), 0, index_path.clone()).unwrap();
            let data_part_1 = vec![
                (Bits::new([0b11111000100010_001000100010001000u32]), 0),
                (Bits::new([0b11001000111110_001000100010001010u32]), 3),
                (Bits::new([0b11111000100010_001000100011111000u32]), 2),
                (Bits::new([0b10011110100010_001000100010001100u32]), 4),
            ];
            index.insert(&data_part_1).unwrap();
            assert_eq!(
                index.data().len(),
                data_part_1.len(),
                "[{}] index length is wrong after first insert",
                i
            );
            let mut expected: Vec<_> = data_part_1.iter().map(|(k, v)| (perm.apply(k), *v)).collect();
            expected.sort_unstable_by_key(|(k, _)| *k);
            assert_eq!(
                index.data(),
                expected,
                "[{}] index contents is wrong after first insert",
                i
            );

            let data_part_2 = vec![
                (Bits::new([0b10001000101110_001000100010001000u32]), 1),
                (Bits::new([0b11111000101110_101000100010001010u32]), 6),
                (Bits::new([0b11111010100010_001000100011111000u32]), 2),
                (Bits::new([0b10010110101110_001000100010001100u32]), 9),
            ];
            index.insert(&data_part_2).unwrap();
            assert_eq!(
                index.data().len(),
                data_part_1.len() + data_part_2.len(),
                "[{}] index length is wrong after second insert",
                i
            );
            expected.extend(data_part_2.iter().map(|(k, v)| (perm.apply(k), *v)));
            expected.sort_unstable_by_key(|(k, _)| *k);
            assert_eq!(
                index.data(),
                expected,
                "[{}] index contents is wrong after second insert",
                i
            );
        }
    }

    #[test]
    fn memmap_index_removal_works_correctly() {
        let tempdir = tempfile::tempdir().expect("failed to create temp dir");
        for (i, perm) in Permutations::get_all_variants().into_iter().enumerate() {
            let index_path = tempdir.path().join("storage.bin");
            let mut index = MemMapIndex::new(perm.clone(), 0, index_path.clone()).unwrap();
            let data = vec![
                (Bits::new([0b11111000100010_001000100010001000u32]), 0),
                (Bits::new([0b11001000111110_001000100010001010u32]), 3),
                (Bits::new([0b11111000100010_001000100011111000u32]), 2),
                (Bits::new([0b10011110100010_001000100010001100u32]), 4),
            ];
            index.insert(&data).unwrap();

            let to_remove = vec![
                Bits::new([0b11111000100010_001000100010001000u32]),
                Bits::new([0b11001000111110_001000100010001010u32]),
            ];
            index.remove(&to_remove).unwrap();
            assert_eq!(
                index.data().len(),
                data.len() - to_remove.len(),
                "[{}] index length is wrong after removal",
                i
            );
            let mut expected: Vec<_> = vec![
                (Bits::new([0b11111000100010_001000100011111000u32]), 2),
                (Bits::new([0b10011110100010_001000100010001100u32]), 4),
            ]
            .iter()
            .map(|(k, v)| (perm.apply(k), *v))
            .collect();
            expected.sort_unstable_by_key(|(k, _)| *k);
            assert_eq!(
                index.data(),
                expected,
                "[{}] index contents is wrong after second insert",
                i
            );
        }
    }
}
