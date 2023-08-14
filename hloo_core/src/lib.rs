mod bit_block;
mod permutations;

pub use bit_block::{BitBlock, BitOp, PermutedBitBlock};
pub use permutations::{create_permutations, Permutation};

pub trait BitContainer {
    type Data;

    /// Get underlying data container.
    fn data(&self) -> &Self::Data;

    /// Get underlying data container.
    fn data_mut(&mut self) -> &mut Self::Data;

    /// Get a single bit value.
    fn bit(&self, idx: usize) -> bool;

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
