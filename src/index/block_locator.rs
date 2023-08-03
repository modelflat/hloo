use std::cmp::Ordering;

use crate::util::exp_search_by;

/// Locates continuous blocks in sorted slice.
#[derive(Clone, Copy, Debug)]
pub enum BlockLocator {
    /// Performs well on any block size.
    DoubleBsearch,
}

impl BlockLocator {
    #[inline(always)]
    pub fn locate_by<'a, T>(&'_ self, data: &'a [T], f: impl Fn(&T) -> Ordering) -> &'a [T] {
        match self {
            BlockLocator::DoubleBsearch => locate_block_double_bsearch(data, f),
        }
    }
}

fn locate_block_double_bsearch<'a, T>(data: &'a [T], f: impl Fn(&T) -> Ordering) -> &'a [T] {
    let maybe_block_start = data.binary_search_by(|el| {
        // 0 0 2 2 2 3 4 5 13
        //     ^st ^end
        f(el).then(Ordering::Greater)
    });
    match maybe_block_start {
        Ok(_) => unreachable!("not possible to find element with a comparator fn that never returns Equals"),
        Err(pos) if pos < data.len() && f(&data[pos]).is_eq() => {
            let block_end = exp_search_by(&data[pos..], |el| {
                // 0 0 2 2 2 3 4 5 13
                //     ^st ^end
                f(el).then(Ordering::Less)
            });
            let block_end = match block_end {
                Ok(_) => unreachable!("not possible to find element with a comparator fn that never returns Equals"),
                Err(pos) => pos,
            };
            &data[pos..(pos + block_end).min(data.len())]
        }
        Err(_) => &data[0..0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locate_block_works_correctly() {
        let data = vec![
            (1u32, 0),
            (2u32, 1),
            (2u32, 2),
            (3u32, 3),
            (4u32, 4),
            (4u32, 5),
            (4u32, 6),
        ];

        let res = locate_block_double_bsearch(&data, |(k, _)| k.cmp(&1));
        assert_eq!(res.len(), 1, "key = 1");
        assert_eq!(res, &data[0..1], "key = 1 - data");
        let res = locate_block_double_bsearch(&data, |(k, _)| k.cmp(&2));
        assert_eq!(res.len(), 2, "key = 2");
        assert_eq!(res, &data[1..3], "key = 2 - data");
        let res = locate_block_double_bsearch(&data, |(k, _)| k.cmp(&3));
        assert_eq!(res.len(), 1, "key = 3");
        assert_eq!(res, &data[3..4], "key = 3 - data");
        let res = locate_block_double_bsearch(&data, |(k, _)| k.cmp(&4));
        assert_eq!(res.len(), 3, "key = 4");
        assert_eq!(res, &data[4..7], "key = 4 - data");
        let res = locate_block_double_bsearch(&data, |(k, _)| k.cmp(&5));
        assert_eq!(res.len(), 0, "key = 5");
        assert_eq!(res, &data[0..0], "key = 5 - data");
        let res = locate_block_double_bsearch(&data, |(k, _)| k.cmp(&0));
        assert_eq!(res.len(), 0, "key = 0");
        assert_eq!(res, &data[0..0], "key = 0 - data");
    }
}
