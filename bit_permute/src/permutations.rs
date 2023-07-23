use std::collections::HashMap;

use itertools::Itertools;

pub use crate::{BitBlock, BitOp, PermutedBitBlock};

pub struct Permutation {
    head: usize,
    blocks: Vec<PermutedBitBlock>,
}

impl Permutation {
    pub fn from_blocks(head: usize, blocks: Vec<BitBlock>) -> Self {
        let permuted_blocks = create_permuted_blocks(&blocks);
        Self {
            head,
            blocks: permuted_blocks,
        }
    }

    pub fn compile_apply(&self, word_size: usize, optimize: bool) -> HashMap<usize, Vec<BitOp>> {
        compile_permutation(
            self.blocks.iter().flat_map(|block| block.to_ops(word_size)),
            word_size,
            optimize,
        )
    }

    pub fn compile_revert(&self, word_size: usize, optimize: bool) -> HashMap<usize, Vec<BitOp>> {
        compile_permutation(
            self.blocks.iter().flat_map(|block| block.apply().to_ops(word_size)),
            word_size,
            optimize,
        )
    }

    pub fn compile_top_mask(&self, word_size: usize, optimize: bool) -> HashMap<usize, Vec<BitOp>> {
        compile_permutation(
            self.blocks
                .iter()
                .take(self.head)
                .flat_map(|block| block.to_ops(word_size)),
            word_size,
            optimize,
        )
    }

    pub fn blocks(&self) -> &[PermutedBitBlock] {
        &self.blocks
    }

    pub fn mask_bits(&self) -> usize {
        self.blocks[..self.head].iter().map(|b| b.block.len()).sum()
    }

    pub fn mask_words(&self, word_size: usize) -> usize {
        let bits = self.mask_bits();
        bits / word_size + if bits % word_size == 0 { 0 } else { 1 }
    }
}

fn compile_permutation<'a>(
    ops: impl Iterator<Item = BitOp>,
    word_size: usize,
    optimize: bool,
) -> HashMap<usize, Vec<BitOp>> {
    let grouped_by_dst_word = ops.group_by(|op| (op.dst_word(), op.src_word()));

    let mut result: HashMap<_, Vec<_>> = HashMap::new();
    for ((dst_word, src_word), mut ops) in grouped_by_dst_word.into_iter() {
        let mut prev_op = ops.next().expect("empty group");
        let mut word_ops = Vec::new();
        for op in ops {
            if optimize {
                if let Some(combined_op) = prev_op.combine(&op) {
                    if combined_op.mask().leading_ones() == word_size as u32 {
                        prev_op = BitOp::Copy { src_word, dst_word }
                    } else {
                        prev_op = combined_op;
                    }
                    continue;
                }
            }
            word_ops.push(prev_op);
            prev_op = op
        }
        word_ops.push(prev_op);
        result
            .entry(dst_word)
            .and_modify(|e| e.extend(&word_ops))
            .or_insert(word_ops);
    }
    result
}

fn split_bits_into_blocks(f: usize, r: usize) -> Vec<BitBlock> {
    assert!(f >= r, "{} is not enough bits to split into {} blocks", f, r);
    let mut blocks = Vec::with_capacity(r);
    let mut acc = 0;
    for i in 0..r {
        let size = f / r + if i < f % r { 1 } else { 0 };
        blocks.push(BitBlock::new(i, acc, size));
        acc += size;
    }
    blocks
}

fn reorder_blocks(blocks: &[BitBlock], order: &[usize]) -> Vec<BitBlock> {
    let mut permuted = Vec::new();
    for pos in order {
        permuted.push(blocks[*pos]);
    }
    permuted.extend(blocks.iter().filter(|block| !order.contains(&block.idx())));
    permuted
}

fn create_permuted_blocks(reordered_blocks: &[BitBlock]) -> Vec<PermutedBitBlock> {
    let mut permuted = Vec::new();
    let mut acc = 0;
    for block in reordered_blocks {
        permuted.push(PermutedBitBlock::new(*block, acc));
        acc += block.len();
    }
    permuted
}

pub fn create_permutations(total_bits: usize, word_bits: usize, r: usize, k: usize) -> Vec<Permutation> {
    assert!(
        total_bits % word_bits == 0,
        "total_bits has to be divisible by word_bits (tb={} wb={})",
        total_bits,
        word_bits
    );
    assert!(
        total_bits >= k,
        "total_bits must be able to fit k (tb={} k={})",
        total_bits,
        k,
    );
    assert!(r != 0 && k != 0, "r and k cannot be 0 (r={} k={})", r, k);
    let blocks = split_bits_into_blocks(total_bits, r);
    (0..r)
        .combinations(k)
        .map(|order| reorder_blocks(&blocks, &order))
        .map(|blocks| Permutation::from_blocks(k, blocks))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_into_blocks() {
        let res = split_bits_into_blocks(64, 5);
        let expected = vec![
            BitBlock::new(0, 0, 13),
            BitBlock::new(1, 13, 13),
            BitBlock::new(2, 26, 13),
            BitBlock::new(3, 39, 13),
            BitBlock::new(4, 52, 12),
        ];
        assert_eq!(res, expected);

        let res = split_bits_into_blocks(64, 4);
        let expected = vec![
            BitBlock::new(0, 0, 16),
            BitBlock::new(1, 16, 16),
            BitBlock::new(2, 32, 16),
            BitBlock::new(3, 48, 16),
        ];
        assert_eq!(res, expected);
    }

    #[test]
    #[should_panic]
    fn test_split_into_blocks_not_enough_bits_panics() {
        let _ = split_bits_into_blocks(8, 12);
    }

    #[test]
    fn test_reorder_blocks() {
        let blocks = vec![
            BitBlock::new(0, 0, 1),
            BitBlock::new(1, 1, 1),
            BitBlock::new(2, 2, 1),
            BitBlock::new(3, 3, 1),
            BitBlock::new(4, 4, 1),
        ];
        let permuted = reorder_blocks(&blocks, &vec![3, 2, 0, 4, 1]);
        assert_eq!(
            permuted,
            vec![
                BitBlock::new(3, 3, 1),
                BitBlock::new(2, 2, 1),
                BitBlock::new(0, 0, 1),
                BitBlock::new(4, 4, 1),
                BitBlock::new(1, 1, 1),
            ]
        );

        let partially_permuted = reorder_blocks(&blocks, &vec![3, 2]);
        assert_eq!(
            partially_permuted,
            vec![
                BitBlock::new(3, 3, 1),
                BitBlock::new(2, 2, 1),
                BitBlock::new(0, 0, 1),
                BitBlock::new(1, 1, 1),
                BitBlock::new(4, 4, 1),
            ]
        );

        let ident = reorder_blocks(&blocks, &vec![0, 1, 2, 3, 4]);
        assert_eq!(ident, blocks);

        let ident2 = reorder_blocks(&blocks, &vec![]);
        assert_eq!(ident2, blocks);
    }

    #[test]
    fn test_create_permuted_blocks() {
        let reordered = vec![
            BitBlock::new(0, 0, 13),
            BitBlock::new(2, 26, 13),
            BitBlock::new(4, 52, 12),
            BitBlock::new(1, 13, 13),
            BitBlock::new(3, 39, 13),
        ];
        let res = create_permuted_blocks(&reordered);
        assert_eq!(
            res,
            vec![
                PermutedBitBlock::new(reordered[0], 0),
                PermutedBitBlock::new(reordered[1], 13),
                PermutedBitBlock::new(reordered[2], 26),
                PermutedBitBlock::new(reordered[3], 38),
                PermutedBitBlock::new(reordered[4], 51),
            ]
        )
    }
}
