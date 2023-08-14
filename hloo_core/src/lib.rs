mod bit_block;
mod permutations;

pub use bit_block::{BitBlock, BitOp, PermutedBitBlock};
pub use permutations::{create_permutations, Permutation};

/// std::ops::Index for bits.
pub trait BitIndex {
    fn index(&self, idx: usize) -> bool;
}

pub trait Distance {
    /// Compute distance as number of different bits between `self` and `other`.
    fn xor_dist(&self, other: &Self) -> u32;
}

pub trait BitPermuter {
    type Bits;
    type Mask;

    /// Apply permutation to bit sequence `key`.
    fn apply(&self, key: &Self::Bits) -> Self::Bits;

    /// Revert permutation of bit sequence `key`.
    fn revert(&self, key: &Self::Bits) -> Self::Bits;

    /// Apply mask to bit sequence `key`.
    fn mask(&self, key: &Self::Bits) -> Self::Mask;

    /// Get number of blocks this permuter operates on
    fn n_blocks(&self) -> u32;
}
