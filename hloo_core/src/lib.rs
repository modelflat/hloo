mod bit_block;
mod permutations;

pub use bit_block::{BitBlock, BitOp, PermutedBitBlock};
pub use permutations::{create_permutations, Permutation};

pub trait BitContainer: Default {
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

pub trait BitPermuter<B, M> {
    /// Apply permutation to bit sequence `key`. Statically dispatched.
    fn apply_static(key: &B) -> B
    where
        Self: Sized;

    /// Apply permutation to bit sequence `key`. Statically dispatched.
    fn revert_static(key: &B) -> B
    where
        Self: Sized;

    /// Apply permutation to bit sequence `key`. Statically dispatched.
    fn mask_static(key: &B) -> M
    where
        Self: Sized;

    /// Apply permutation to bit sequence `key`.
    fn apply(&self, key: &B) -> B;

    /// Revert permutation of bit sequence `key`.
    fn revert(&self, key: &B) -> B;

    /// Apply mask to bit sequence `key`.
    fn mask(&self, key: &B) -> M;

    /// Get number of blocks this permuter operates on.
    fn n_blocks(&self) -> u32;
}
