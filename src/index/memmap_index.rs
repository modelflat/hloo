use std::{marker::PhantomData, path::PathBuf};

use bit_permute::Distance;
use thiserror::Error;

use crate::mmvec::{MmVec, MmVecError};

use super::{compute_index_stats, scan_block, select_block_at, BitPermuter, Index, SearchResultItem};

#[derive(Debug, Error)]
pub enum MemMapIndexError {
    #[error("distance ({distance}) exceeds maximum allowed distance for index ({max})")]
    DistanceExceedsMax { distance: u32, max: u32 },
    #[error("mmvec error: {0}")]
    IoError(#[from] MmVecError),
}

pub struct MemMapIndex<K, V, M, P> {
    permuter: P,
    path: PathBuf,
    data_signature: u64,
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
    pub fn new(permuter: P, sig: u64, path: PathBuf) -> Result<Self, MemMapIndexError> {
        Ok(Self {
            permuter,
            path: path.clone(),
            data_signature: sig,
            data: MmVec::new_empty(sig, path)?,
            mask_type: Default::default(),
        })
    }

    pub fn data(&self) -> &[(K, V)] {
        unsafe { self.data.as_slice() }
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

    fn load(&mut self) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        self.data = MmVec::from_path(self.data_signature, self.path.clone())?;
        Ok(())
    }

    fn save(&self) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        self.data.flush()?;
        Ok(())
    }

    fn insert(&mut self, items: &[(K, V)]) -> Result<(), Self::Error> {
        let mut permuted: Vec<_> = items.iter().map(|(k, v)| (self.permuter.apply(*k), *v)).collect();
        permuted.sort_by_key(|(k, _)| *k);
        self.data.merge_sorted(&permuted, |(k, _)| k)?;
        Ok(())
    }

    fn search(&self, key: K, distance: u32) -> Result<Vec<SearchResultItem<V>>, Self::Error> {
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

#[cfg(test)]
mod tests {
    use bit_permute::{BitPermuter, Distance, DynBitPermuter};
    use bit_permute_macro::make_permutations;

    use super::*;

    make_permutations!(struct_name = "Permutations", f = 32, r = 5, k = 2, w = 32);
    // blocks: 7 7 6 6 6
    // mask width: 32 / 5 ; 2 -> 14

    #[test]
    fn test_memvec_index_search_works() {
        let tempdir = tempfile::tempdir().expect("failed to create temp dir");
        let mut index = MemMapIndex::new(Permutations::get_variant(0), 0, tempdir.path().join("storage.bin"))
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
