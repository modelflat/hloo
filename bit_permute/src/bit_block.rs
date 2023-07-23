/// Returns a bit mask of length `len` starting at a given bit `pos`.
fn compute_mask(pos: usize, len: usize, word_size: usize) -> u64 {
    assert!(
        0 < word_size && word_size <= 64,
        "word size {} is not supported",
        word_size
    );
    assert!(
        pos + len <= word_size,
        "invalid values for mask: len={} pos={} (len + pos = {}, which is to big for a {}-bit word)",
        len,
        pos,
        pos + len,
        word_size
    );
    ((1 << len) - 1) << pos
}

/// Restores pos and len from a mask.
fn unmask(mask: u64) -> (usize, usize) {
    let pos = mask.trailing_zeros();
    let len = (mask >> pos).trailing_ones();
    (pos as usize, len as usize)
}

fn combine_masks(m1: u64, m2: u64) -> Option<u64> {
    let (pos1, len1) = unmask(m1);
    let (pos2, len2) = unmask(m2);
    if pos1 + len1 == pos2 || pos2 + len2 == pos1 {
        Some(m1 | m2)
    } else {
        None
    }
}

/// Represents a range of bits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BitBlock {
    idx: usize,
    pos: usize,
    len: usize,
}

impl BitBlock {
    pub fn new(idx: usize, pos: usize, len: usize) -> Self {
        assert_ne!(len, 0, "block can't be of length 0!");
        Self { idx, pos, len }
    }

    /// Index of this block
    pub fn idx(&self) -> usize {
        self.idx
    }

    /// Global number of a bit this block starts at.
    pub fn start_pos(&self) -> usize {
        self.pos
    }

    /// Global number of a bit this block ends at.
    pub fn end_pos(&self) -> usize {
        self.pos + self.len - 1
    }

    /// Length of this block in bits
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Index of a word this block starts at.
    pub fn start_word(&self, word_size: usize) -> usize {
        self.start_pos() / word_size
    }

    /// Index of a word this block ends at.
    pub fn end_word(&self, word_size: usize) -> usize {
        self.end_pos() / word_size
    }

    /// Index of a bit this block is located at within its last word.
    ///
    /// Note that bits within a word are enumerated in reverse order: `[ bN-1, bN-2, ... b0 ]`.
    pub fn end_bit(&self, word_size: usize) -> usize {
        word_size - 1 - self.end_pos() % word_size
    }

    /// Whether this block resides entirely within a single word
    pub fn is_contiguous(&self, word_size: usize) -> bool {
        self.start_word(word_size) == self.end_word(word_size)
    }

    /// Length of this block in words
    pub fn len_words(&self, word_size: usize) -> usize {
        let rem = if self.len() % word_size == 0 { 0 } else { 1 };
        self.len() / word_size + rem
    }

    /// Split this block by word boundaries
    pub fn split(&self, word_size: usize) -> Vec<Self> {
        let start_word = self.start_pos() / word_size;
        let end_word = self.end_pos() / word_size;
        let mut parts = Vec::new();
        for word_idx in start_word..=end_word {
            let word_start_idx = word_idx * word_size;
            let word_end_idx = (word_idx + 1) * word_size - 1;
            let start_idx = self.start_pos().max(word_start_idx);
            let end_idx = self.end_pos().min(word_end_idx);
            let part_len = end_idx - start_idx + 1;
            parts.push(BitBlock::new(self.idx, start_idx, part_len))
        }
        parts
    }

    /// Move this block to the new position, respecting both old and new word boundaries
    pub fn move_to(&self, new_pos: usize, word_size: usize) -> Vec<(Self, Vec<Self>)> {
        let mut part_pos = new_pos;
        let mut new_parts = Vec::new();
        for part in self.split(word_size) {
            let moved_part = BitBlock::new(part.idx, part_pos, part.len());
            new_parts.push((part, moved_part.split(word_size)));
            part_pos += part.len();
        }
        new_parts
    }

    /// If a block is a single-word block, return the bit it is located at within the word; otherwise None.
    pub fn bit_pos(&self, word_size: usize) -> Option<usize> {
        if self.is_contiguous(word_size) {
            Some(self.end_bit(word_size))
        } else {
            None
        }
    }

    /// If a block is a single-word block, return the its corresponding bit mask; otherwise None.
    pub fn mask(&self, word_size: usize) -> Option<u64> {
        self.bit_pos(word_size)
            .map(|bit| compute_mask(bit, self.len(), word_size))
    }

    /// If a block is a single-word block, return its word and mask; otherwise None.
    pub fn coord(&self, word_size: usize) -> Option<(usize, usize)> {
        self.bit_pos(word_size).map(|bit| (self.end_word(word_size), bit))
    }
}

/// Low-level bit operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BitOp {
    MaskShiftAndCopy {
        src_word: usize,
        src_mask: u64,
        src_shift: i64,
        dst_word: usize,
    },
    MaskAndCopy {
        src_word: usize,
        src_mask: u64,
        dst_word: usize,
    },
    Copy {
        src_word: usize,
        dst_word: usize,
    },
}

impl BitOp {
    pub fn copy_block(src: BitBlock, dst: BitBlock, word_size: usize) -> Self {
        assert_eq!(
            src.len(),
            dst.len(),
            "src and should be the same size! {} vs {}",
            src.len(),
            dst.len()
        );
        let (src_word, src_bit) = src.coord(word_size).expect("expected a unit block");
        let (dst_word, dst_bit) = dst.coord(word_size).expect("expected a unit block");
        let src_mask = src.mask(word_size).expect("expected a unit block");
        if src_bit == dst_bit {
            if src.len() == word_size {
                Self::Copy { src_word, dst_word }
            } else {
                Self::MaskAndCopy {
                    src_word,
                    src_mask,
                    dst_word,
                }
            }
        } else {
            Self::MaskShiftAndCopy {
                src_word,
                src_mask,
                src_shift: dst_bit as i64 - src_bit as i64,
                dst_word,
            }
        }
    }

    pub fn mask_block(src: BitBlock, word_size: usize) -> Self {
        let word = src.start_word(word_size);
        let mask = src.mask(word_size).expect("expected a unit block");
        Self::MaskAndCopy {
            src_word: word,
            src_mask: mask,
            dst_word: word,
        }
    }

    pub fn src_word(&self) -> usize {
        match self {
            Self::MaskShiftAndCopy { src_word, .. } => *src_word,
            Self::MaskAndCopy { src_word, .. } => *src_word,
            Self::Copy { src_word, .. } => *src_word,
        }
    }

    pub fn dst_word(&self) -> usize {
        match self {
            Self::MaskShiftAndCopy { dst_word, .. } => *dst_word,
            Self::MaskAndCopy { dst_word, .. } => *dst_word,
            Self::Copy { dst_word, .. } => *dst_word,
        }
    }

    pub fn shift(&self) -> i64 {
        match self {
            Self::MaskShiftAndCopy { src_shift, .. } => *src_shift,
            _ => 0,
        }
    }

    pub fn mask(&self) -> u64 {
        match self {
            Self::MaskShiftAndCopy { src_mask, .. } => *src_mask,
            Self::MaskAndCopy { src_mask, .. } => *src_mask,
            Self::Copy { .. } => u64::MAX,
        }
    }

    fn set_mask(&mut self, mask: u64) -> Self {
        match self {
            Self::MaskShiftAndCopy { src_mask, .. } => *src_mask = mask,
            Self::MaskAndCopy { src_mask, .. } => *src_mask = mask,
            Self::Copy { .. } => {}
        }
        *self
    }

    pub fn clone_with_mask(&self, mask: u64) -> Self {
        self.clone().set_mask(mask)
    }

    pub fn combine(&self, op: &Self) -> Option<Self> {
        if self.clone_with_mask(0) == op.clone_with_mask(0) {
            if let Some(combined_mask) = combine_masks(self.mask(), op.mask()) {
                Some(self.clone_with_mask(combined_mask))
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl std::fmt::Display for BitOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fmt_width = 32;
        match self {
            Self::MaskShiftAndCopy {
                src_word,
                src_mask,
                src_shift,
                dst_word,
            } => write!(
                f,
                "a[{}] = ( a[{}] & {:0width$b} ) {} {:02}",
                dst_word,
                src_word,
                src_mask,
                if *src_shift < 0 { ">>" } else { "<<" },
                src_shift.abs(),
                width = fmt_width
            ),
            Self::MaskAndCopy {
                src_word,
                src_mask,
                dst_word,
            } => write!(
                f,
                "a[{}] = ( a[{}] & {:0width$b} )",
                dst_word,
                src_word,
                src_mask,
                width = fmt_width
            ),
            Self::Copy { src_word, dst_word } => write!(f, "a[{}] = a[{}]", dst_word, src_word),
        }
    }
}

/// Represents a range of bits which have been moved into a new position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PermutedBitBlock {
    pub block: BitBlock,
    pub new_pos: usize,
}

impl PermutedBitBlock {
    pub fn new(block: BitBlock, new_pos: usize) -> Self {
        Self { block, new_pos }
    }

    pub fn apply(&self) -> Self {
        PermutedBitBlock {
            block: BitBlock::new(self.block.idx(), self.new_pos, self.block.len()),
            new_pos: self.block.start_pos(),
        }
    }

    pub fn to_ops(&self, word_size: usize) -> Vec<BitOp> {
        let moved_parts = self.block.move_to(self.new_pos, word_size);
        let mut ops = Vec::new();
        for (src, dst_parts) in moved_parts {
            let mut src_pos = src.start_pos();
            for dst in dst_parts {
                let src_sub = BitBlock::new(src.idx(), src_pos, dst.len());
                let op = BitOp::copy_block(src_sub, dst, word_size);
                ops.push(op);
                src_pos += dst.len();
            }
        }
        ops
    }

    pub fn to_mask_ops(&self, word_size: usize) -> Vec<BitOp> {
        self.apply()
            .block
            .split(word_size)
            .into_iter()
            .map(|b| BitOp::mask_block(b, word_size))
            .collect()
    }
}

impl std::fmt::Display for PermutedBitBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.block.start_pos() == self.new_pos {
            write!(
                f,
                "Block {:03}-{:03} (not moved) ({:2})",
                self.block.start_pos() + self.block.len(),
                self.block.start_pos(),
                self.block.len()
            )
        } else {
            write!(
                f,
                "Block {:03}-{:03} => {:03}-{:03} ({:2})",
                self.block.start_pos() + self.block.len(),
                self.block.start_pos(),
                self.new_pos + self.block.len(),
                self.new_pos,
                self.block.len()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_mask() {
        let mask = compute_mask(0, 5, 64);
        assert_eq!(mask, 0b11111);
        let mask = compute_mask(5, 5, 64);
        assert_eq!(mask, 0b1111100000);
        let mask = compute_mask(0, 1, 64);
        assert_eq!(mask, 0b1);
        let mask = compute_mask(63, 1, 64);
        assert_eq!(mask, 0b1000000000000000000000000000000000000000000000000000000000000000);
    }

    #[test]
    fn test_unmask() {
        let res = unmask(0b11111);
        assert_eq!(res, (0, 5));
        let res = unmask(0b1111100000);
        assert_eq!(res, (5, 5));
        let res = unmask(0b1);
        assert_eq!(res, (0, 1));
        let res = unmask(0b1000000000000000000000000000000000000000000000000000000000000000);
        assert_eq!(res, (63, 1));
    }

    #[test]
    fn test_combine_masks() {
        let m1 = 0b00001110;
        let m2 = 0b00110000;
        assert_eq!(combine_masks(m1, m2), Some(0b00111110));

        let m1 = 0b00000001;
        let m2 = 0b11111110;
        assert_eq!(combine_masks(m1, m2), Some(0b11111111));

        let m1 = 0b00001110;
        let m2 = 0b00100000;
        assert_eq!(combine_masks(m1, m2), None);

        let m1 = 0b00001110;
        let m2 = 0b00111100;
        assert_eq!(combine_masks(m1, m2), None);

        let m1 = 0b00001110;
        let m2 = 0b00001110;
        assert_eq!(combine_masks(m1, m2), None);
    }

    #[test]
    fn test_compute_mask_word_sizes() {
        let mask = compute_mask(4, 3, 64);
        assert_eq!(mask, 0b1110000);
        let mask = compute_mask(4, 3, 39);
        assert_eq!(mask, 0b1110000);
        let mask = compute_mask(4, 3, 32);
        assert_eq!(mask, 0b1110000);
        let mask = compute_mask(4, 3, 24);
        assert_eq!(mask, 0b1110000);
        let mask = compute_mask(4, 3, 7);
        assert_eq!(mask, 0b1110000);
    }

    #[test]
    #[should_panic]
    fn test_compute_mask_panics_when_mask_does_not_fit() {
        let _ = compute_mask(1, 8, 8);
    }

    #[test]
    fn test_block_word() {
        // ....|.+++|++++|+...
        let block = BitBlock::new(0, 5, 8);
        let word_size = 4;
        assert_eq!(block.start_word(word_size), 1, "start_word 4");
        assert_eq!(block.end_word(word_size), 3, "end_word 4");
        let word_size = 16;
        assert_eq!(block.start_word(word_size), 0, "start_word 16");
        assert_eq!(block.end_word(word_size), 0, "end_word 16");
    }

    #[test]
    fn test_block_coord() {
        // ....|.+++|++++|+...
        let block = BitBlock::new(0, 5, 8);
        let word_size = 4;
        assert_eq!(block.start_word(word_size), 1, "start_word 4");
        assert_eq!(block.end_word(word_size), 3, "end_word 4");
        assert_eq!(block.end_bit(word_size), 3, "end_bit 4");

        // ++++++++++++++++
        let block = BitBlock::new(0, 0, 16);
        let word_size = 16;
        assert_eq!(block.start_word(word_size), 0, "start_word 16");
        assert_eq!(block.end_word(word_size), 0, "end_word 16");
        assert_eq!(block.end_bit(word_size), 0, "end_bit 16");

        // ........|+++++++.
        let block = BitBlock::new(0, 8, 7);
        let word_size = 8;
        assert_eq!(block.start_word(word_size), 1, "start_word 8");
        assert_eq!(block.end_word(word_size), 1, "end_word 8");
        assert_eq!(block.end_bit(word_size), 1, "end_bit 8");
    }

    #[test]
    fn test_block_len_words() {
        let block = BitBlock::new(0, 0, 16);
        assert_eq!(block.len_words(4), 4);
        assert_eq!(block.len_words(8), 2);
        assert_eq!(block.len_words(32), 1);
        assert_eq!(block.len_words(64), 1);

        let block = BitBlock::new(0, 15, 16);
        assert_eq!(block.len_words(4), 4);
        assert_eq!(block.len_words(8), 2);
        assert_eq!(block.len_words(32), 1);
        assert_eq!(block.len_words(64), 1);
    }

    #[test]
    fn test_block_split() {
        // ....|..++|++++|+...
        let block = BitBlock::new(0, 6, 7);
        let parts = block.split(4);
        assert_eq!(
            parts,
            vec![
                // word end block
                BitBlock::new(0, 6, 2),
                // full block
                BitBlock::new(0, 8, 4),
                // word start block
                BitBlock::new(0, 12, 1),
            ]
        );

        for unit in parts {
            assert!(unit.is_contiguous(4));
            assert_eq!(unit.split(4), vec![unit]);
        }
    }

    #[test]
    fn test_block_move() {
        // ....|..++|++++|+...
        //      moved to
        // ....|++++|+++.|....
        //    final split
        // ....|++;++|++;+.|....
        let block = BitBlock::new(0, 6, 7);
        let parts = block.split(4);
        let moved_parts = block.move_to(4, 4);
        assert_eq!(
            moved_parts,
            vec![
                (parts[0], vec![BitBlock::new(0, 4, 2)]),
                (parts[1], vec![BitBlock::new(0, 6, 2), BitBlock::new(0, 8, 2)]),
                (parts[2], vec![BitBlock::new(0, 10, 1)]),
            ]
        );

        for (src, moved) in moved_parts {
            assert_eq!(src.len(), moved.iter().map(|b| b.len()).sum());
            for unit in moved {
                assert!(unit.is_contiguous(4));
                assert_eq!(unit.split(4), vec![unit]);
            }
        }
    }

    #[test]
    fn test_block_to_ops() {
        // ......++|+++++...
        //      moved to
        // ....++++|+++.....
        let word_size = 8;
        let block = PermutedBitBlock::new(BitBlock::new(0, 6, 7), 4);
        let ops = block.to_ops(word_size);
        let expected = vec![
            BitOp::copy_block(BitBlock::new(0, 6, 2), BitBlock::new(0, 4, 2), word_size),
            BitOp::copy_block(BitBlock::new(0, 8, 2), BitBlock::new(0, 6, 2), word_size),
            BitOp::copy_block(BitBlock::new(0, 10, 3), BitBlock::new(0, 8, 3), word_size),
        ];
        assert_eq!(ops, expected);
    }

    #[test]
    fn test_block_to_ops_contiguous() {
        // ......+++++++...
        //      moved to
        // ....+++++++.....
        let word_size = 16;
        let block = PermutedBitBlock::new(BitBlock::new(0, 6, 7), 4);
        let ops = block.to_ops(word_size);
        let expected = vec![BitOp::copy_block(
            BitBlock::new(0, 6, 7),
            BitBlock::new(0, 4, 7),
            word_size,
        )];
        assert_eq!(ops, expected);
    }

    #[test]
    fn test_materialized_ops_1() {
        // ......++|++;+++...
        // ....++;++|+++.....
        let word_size = 8;
        let copy_1 = BitOp::copy_block(BitBlock::new(0, 6, 2), BitBlock::new(0, 4, 2), word_size);
        let expected_1 = BitOp::MaskShiftAndCopy {
            src_word: 0,
            src_mask: 0b11,
            src_shift: 2,
            dst_word: 0,
        };
        assert_eq!(copy_1, expected_1, "copy first block");

        let copy_2 = BitOp::copy_block(BitBlock::new(0, 8, 2), BitBlock::new(0, 6, 2), word_size);
        let expected_2 = BitOp::MaskShiftAndCopy {
            src_word: 1,
            src_mask: 0b11000000,
            src_shift: -6,
            dst_word: 0,
        };
        assert_eq!(copy_2, expected_2, "copy second block");

        let copy_3 = BitOp::copy_block(BitBlock::new(0, 10, 3), BitBlock::new(0, 8, 3), word_size);
        let expected_3 = BitOp::MaskShiftAndCopy {
            src_word: 1,
            src_mask: 0b00111000,
            src_shift: 2,
            dst_word: 1,
        };
        assert_eq!(copy_3, expected_3, "copy third block");
    }

    #[test]
    fn test_materialized_ops_2() {
        // ....++;++|+++++...|........
        // ........|......++|++;+++++.
        let word_size = 8;
        let copy_1 = BitOp::copy_block(BitBlock::new(0, 4, 2), BitBlock::new(0, 14, 2), word_size);
        let expected_1 = BitOp::MaskShiftAndCopy {
            src_word: 0,
            src_mask: 0b1100,
            src_shift: -2,
            dst_word: 1,
        };
        assert_eq!(copy_1, expected_1, "copy first block");

        let copy_2 = BitOp::copy_block(BitBlock::new(0, 6, 2), BitBlock::new(0, 16, 2), word_size);
        let expected_2 = BitOp::MaskShiftAndCopy {
            src_word: 0,
            src_mask: 0b11,
            src_shift: 6,
            dst_word: 2,
        };
        assert_eq!(copy_2, expected_2, "copy second block");

        let copy_3 = BitOp::copy_block(BitBlock::new(0, 8, 5), BitBlock::new(0, 18, 5), word_size);
        let expected_3 = BitOp::MaskShiftAndCopy {
            src_word: 1,
            src_mask: 0b11111000,
            src_shift: -2,
            dst_word: 2,
        };
        assert_eq!(copy_3, expected_3, "copy third block");
    }
}
