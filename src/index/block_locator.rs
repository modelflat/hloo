use std::cmp::Ordering;

use crate::util::exp_search;

#[derive(Clone, Copy, Debug)]
pub enum BlockLocator {
    /// Can sometimes do better than DoubleBsearch on very small block sizes.
    Naive,
    /// Performs well on any block size.
    DoubleBsearch,
}

impl BlockLocator {
    #[inline(always)]
    pub fn locate<'a, K, V, M>(&'_ self, data: &'a [(K, V)], key: &'_ K, mask_fn: impl Fn(&K) -> M) -> &'a [(K, V)]
    where
        M: Ord,
    {
        match self {
            BlockLocator::Naive => locate_block_bsearch_and_naive_select(data, key, mask_fn),
            BlockLocator::DoubleBsearch => locate_block_double_bsearch(data, key, mask_fn),
        }
    }
}

fn locate_block_double_bsearch<'a, K, V, M>(data: &'a [(K, V)], key: &'_ K, mask_fn: impl Fn(&K) -> M) -> &'a [(K, V)]
where
    M: Ord,
{
    let masked_key = mask_fn(key);
    let maybe_block_start = data.binary_search_by(|(key, _)| {
        // 0 0 2 2 2 3 4 5 13
        //     ^st ^end
        if mask_fn(key) < masked_key {
            Ordering::Less
        } else {
            // NOTE: equal values are also considered greater
            Ordering::Greater
        }
    });
    match maybe_block_start {
        Ok(_) => unreachable!("not possible to find element with a comparator fn that never returns Equals"),
        Err(pos) if pos < data.len() && mask_fn(&data[pos].0) == masked_key => {
            let block_end = exp_search(&data[pos..], |(key, _)| {
                // 0 0 2 2 2 3 4 5 13
                //     ^st ^end
                if mask_fn(key) > masked_key {
                    Ordering::Greater
                } else {
                    // NOTE: equal values are also considered less
                    Ordering::Less
                }
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

fn locate_block_bsearch_and_naive_select<'a, K, V, M>(
    data: &'a [(K, V)],
    key: &'_ K,
    mask_fn: impl Fn(&K) -> M,
) -> &'a [(K, V)]
where
    M: Ord,
{
    let masked_key = mask_fn(key);
    let location = data.binary_search_by_key(&masked_key, |(key, _)| mask_fn(key));
    match location {
        Ok(pos) => select_block_at(data, pos, mask_fn),
        Err(_) => &data[0..0],
    }
}

fn select_block_at<K, V, M>(data: &[(K, V)], pos: usize, mask_fn: impl Fn(&K) -> M) -> &[(K, V)]
where
    M: Ord,
{
    let key = mask_fn(&data[pos].0);
    let mut start = pos;
    while start > 0 && key == mask_fn(&data[start - 1].0) {
        start -= 1;
    }
    let mut end = pos;
    while end < data.len() - 1 && key == mask_fn(&data[end + 1].0) {
        end += 1;
    }
    &data[start..=end]
}

#[cfg(test)]
mod tests {
    use bit_permute::Distance;

    use crate::index::{scan_block, SearchResultItem};

    use super::*;

    fn id<T: Copy + Ord>(x: &T) -> T {
        *x
    }

    struct MyKey(u32);

    impl Distance for MyKey {
        fn xor_dist(&self, other: &Self) -> u32 {
            self.0.abs_diff(other.0)
        }
    }

    #[test]
    fn test_scan_block_works_correctly() {
        let data = vec![
            (MyKey(1u32), 0),
            (MyKey(2u32), 1),
            (MyKey(2u32), 2),
            (MyKey(3u32), 3),
            (MyKey(4u32), 4),
            (MyKey(4u32), 5),
            (MyKey(4u32), 6),
        ];

        let res = scan_block(&data, &MyKey(1), 0);
        assert_eq!(res.len(), 1, "pos 0");
        assert_eq!(res, vec![SearchResultItem::new(0, 0)], "pos 0 - data");
        let res = scan_block(&data, &MyKey(1), 1);
        assert_eq!(res.len(), 3, "pos 0-2");
        assert_eq!(
            res,
            vec![
                SearchResultItem::new(0, 0),
                SearchResultItem::new(1, 1),
                SearchResultItem::new(2, 1),
            ],
            "pos 0-2 - data"
        )
    }

    #[test]
    fn test_select_block_at_works_correctly() {
        let data = vec![
            (1u32, 0),
            (2u32, 1),
            (2u32, 2),
            (3u32, 3),
            (4u32, 4),
            (4u32, 5),
            (4u32, 6),
        ];

        let result0 = select_block_at(&data, 0, id);
        assert_eq!(result0.len(), 1, "pos 0");
        assert_eq!(result0, &data[0..1], "pos 0 - data");
        let result0 = select_block_at(&data, 1, id);
        assert_eq!(result0.len(), 2, "pos 1");
        assert_eq!(result0, &data[1..3], "pos 1 - data");
        let result0 = select_block_at(&data, 2, id);
        assert_eq!(result0.len(), 2, "pos 2");
        assert_eq!(result0, &data[1..3], "pos 2 - data");
        let result0 = select_block_at(&data, 3, id);
        assert_eq!(result0.len(), 1, "pos 3");
        assert_eq!(result0, &data[3..4], "pos 3 - data");
        let result0 = select_block_at(&data, 4, id);
        assert_eq!(result0.len(), 3, "pos 4");
        assert_eq!(result0, &data[4..7], "pos 4 - data");
        let result0 = select_block_at(&data, 5, id);
        assert_eq!(result0.len(), 3, "pos 5");
        assert_eq!(result0, &data[4..7], "pos 5 - data");
        let result0 = select_block_at(&data, 6, id);
        assert_eq!(result0.len(), 3, "pos 6");
        assert_eq!(result0, &data[4..7], "pos 6 - data");
    }

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

        let res = locate_block_double_bsearch(&data, &1, id);
        assert_eq!(res.len(), 1, "key = 1");
        assert_eq!(res, &data[0..1], "key = 1 - data");
        let res = locate_block_double_bsearch(&data, &2, id);
        println!("{:?}", res);
        assert_eq!(res.len(), 2, "key = 2");
        assert_eq!(res, &data[1..3], "key = 2 - data");
        let res = locate_block_double_bsearch(&data, &3, id);
        assert_eq!(res.len(), 1, "key = 3");
        assert_eq!(res, &data[3..4], "key = 3 - data");
        let res = locate_block_double_bsearch(&data, &4, id);
        assert_eq!(res.len(), 3, "key = 4");
        assert_eq!(res, &data[4..7], "key = 4 - data");
        let res = locate_block_double_bsearch(&data, &5, id);
        assert_eq!(res.len(), 0, "key = 5");
        assert_eq!(res, &data[0..0], "key = 5 - data");
        let res = locate_block_double_bsearch(&data, &0, id);
        assert_eq!(res.len(), 0, "key = 0");
        assert_eq!(res, &data[0..0], "key = 0 - data");
    }
}
