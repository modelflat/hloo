use std::{
    any::TypeId,
    cmp::Ordering,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

/// Partition the slice according to the given predicate.
///
/// Elements for which the predicate returns `true` are placed at the start of the slice.
pub fn partition<T, F>(data: &mut [T], predicate: F) -> usize
where
    F: Fn(&T) -> bool,
{
    data.sort_by_key(|el| !predicate(el));
    data.partition_point(predicate)
}

/// Search the slice using binary search with the given comparator. Return a slice starting at the first index for
/// which the comparator returns `Ordering::Equal`, and ending at the last such index (inclusive). If the comparator
/// never returns `Ordering::Equal`, return an empty slice.
pub fn extended_binary_search_by<T>(slice: &[T], f: impl Fn(&T) -> Ordering) -> &[T] {
    // perform the first two steps of the binary search manually to get rid of OOB values right away
    // this may be helpful with some of the skew cases, and makes this search more robust against user-provided data
    let mid = slice.len() / 2;
    let slice = if f(&slice[mid]).then(Ordering::Greater) == Ordering::Greater {
        if f(&slice[0]) == Ordering::Greater {
            // not in bounds
            return &slice[0..0];
        }
        &slice[..=mid]
    } else if f(&slice[slice.len() - 1]) == Ordering::Less {
        // not in bounds
        return &slice[0..0];
    } else {
        &slice[mid..]
    };

    let maybe_block_start = slice.binary_search_by(|el| {
        // 0 0 2 2 2 3 4 5 13
        //     ^st ^end
        f(el).then(Ordering::Greater)
    });

    match maybe_block_start {
        Ok(_) => unreachable!("not possible to find an element with a comparator fn that never returns Equals"),
        Err(pos) if pos < slice.len() && f(&slice[pos]).is_eq() => {
            // exp_search performs best when blocks are small, otherwise binary_search is better
            let block_end = exponential_search_by(&slice[pos..], |el| {
                // 0 0 2 2 2 3 4 5 13
                //     ^st ^end
                f(el).then(Ordering::Less)
            });
            match block_end {
                Ok(_) => unreachable!("not possible to find an element with a comparator fn that never returns Equals"),
                Err(block_end) => &slice[pos..(pos + block_end).min(slice.len())],
            }
        }
        Err(_) => &slice[0..0],
    }
}

/// Perform an exponential binary search over the slice.
fn exponential_search_by<T, F>(slice: &[T], f: F) -> Result<usize, usize>
where
    F: Fn(&T) -> Ordering,
{
    let mut bound = 1;
    while bound < slice.len() && matches!(f(&slice[bound]), Ordering::Less) {
        bound <<= 1;
    }
    let start = bound >> 1;
    slice[start..slice.len().min(bound + 1)]
        .binary_search_by(f)
        .map(|i| i + start)
        .map_err(|i| i + start)
}

/// Create a u64 signature for a given type and permutation parameters.
pub fn sign_type<T: 'static>(f: u64, r: u64, k: u64, w: u64) -> u64 {
    let t = TypeId::of::<T>();
    let mut hasher = DefaultHasher::new();
    t.hash(&mut hasher);
    hasher.write_u64(f);
    hasher.write_u64(r);
    hasher.write_u64(k);
    hasher.write_u64(w);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partition_vector() {
        let mut data = vec![0, 3, 4, 6, 3];
        let split = partition(&mut data, |el| *el != 3);
        assert_eq!(split, 3, "wrong split value");
        assert_eq!(data, vec![0, 4, 6, 3, 3], "wrong partitioned data: {data:?}");
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

        let res = extended_binary_search_by(&data, |(k, _)| k.cmp(&1));
        assert_eq!(res.len(), 1, "key = 1");
        assert_eq!(res, &data[0..1], "key = 1 - data");
        let res = extended_binary_search_by(&data, |(k, _)| k.cmp(&2));
        assert_eq!(res.len(), 2, "key = 2");
        assert_eq!(res, &data[1..3], "key = 2 - data");
        let res = extended_binary_search_by(&data, |(k, _)| k.cmp(&3));
        assert_eq!(res.len(), 1, "key = 3");
        assert_eq!(res, &data[3..4], "key = 3 - data");
        let res = extended_binary_search_by(&data, |(k, _)| k.cmp(&4));
        assert_eq!(res.len(), 3, "key = 4");
        assert_eq!(res, &data[4..7], "key = 4 - data");
        let res = extended_binary_search_by(&data, |(k, _)| k.cmp(&5));
        assert_eq!(res.len(), 0, "key = 5");
        assert_eq!(res, &data[0..0], "key = 5 - data");
        let res = extended_binary_search_by(&data, |(k, _)| k.cmp(&0));
        assert_eq!(res.len(), 0, "key = 0");
        assert_eq!(res, &data[0..0], "key = 0 - data");
    }

    #[test]
    fn exponential_search_works_correctly() {
        let data = vec![0, 3, 4, 6, 7];
        let res = exponential_search_by(&data, |el| el.cmp(&0));
        assert_eq!(res, Ok(0), "0");
        let res = exponential_search_by(&data, |el| el.cmp(&3));
        assert_eq!(res, Ok(1), "3");
        let res = exponential_search_by(&data, |el| el.cmp(&5));
        assert_eq!(res, Err(3), "5");
        let res = exponential_search_by(&data, |el| el.cmp(&1000));
        assert_eq!(res, Err(5), "1000");
        let res = exponential_search_by(&data, |el| el.cmp(&-1000));
        assert_eq!(res, Err(0), "-1000");
        let res = exponential_search_by(&data[0..0], |_| panic!("this should not be called"));
        assert_eq!(res, Err(0), "empty");
    }
}
