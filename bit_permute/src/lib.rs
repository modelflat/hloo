mod bit_block;
mod permutations;

pub use bit_block::{BitBlock, BitOp, PermutedBitBlock};
pub use permutations::{create_permutations, Permutation};

/// std::ops::Index for bits.
pub trait BitIndex<I> {
    fn index(&self, idx: I) -> bool;
}

pub trait BitPermuter<K, M> {
    /// Apply permutation to bit sequence `key`.
    fn apply(&self, key: &K) -> K;

    /// Revert permutation of bit sequence `key`.
    fn revert(&self, key: &K) -> K;

    /// Apply mask to bit sequence `key`.
    fn mask(&self, key: &K) -> M;

    /// Get number of blocks this permuter operates on
    fn n_blocks(&self) -> u32;

    /// Get number of mask bits this permuter has
    fn mask_bits(&self) -> u32;
}

#[derive(Clone)]
pub struct DynBitPermuter<K, M>(pub std::sync::Arc<dyn BitPermuter<K, M>>);

impl<K, M> BitPermuter<K, M> for DynBitPermuter<K, M> {
    fn apply(&self, key: &K) -> K {
        self.0.apply(key)
    }

    fn revert(&self, key: &K) -> K {
        self.0.revert(key)
    }

    fn mask(&self, key: &K) -> M {
        self.0.mask(key)
    }

    fn n_blocks(&self) -> u32 {
        self.0.n_blocks()
    }

    fn mask_bits(&self) -> u32 {
        self.0.mask_bits()
    }
}

pub trait Distance {
    /// Compute distance as number of different bits between `self` and `other`.
    fn xor_dist(&self, other: &Self) -> u32;
}
