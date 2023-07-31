use std::cmp::Ordering;

/// Partition a slice according to predicate.
///
/// Elements for which predicate returns `true` go to the start of the slice.
pub fn partition<T, F>(data: &mut [T], predicate: F) -> usize
where
    F: Fn(&T) -> bool,
{
    data.sort_by_key(|el| !predicate(el));
    data.partition_point(predicate)
}

/// Performs exponential search.
pub fn exp_search<T, F>(data: &[T], f: F) -> Result<usize, usize>
where
    F: Fn(&T) -> Ordering,
{
    let mut bound = 1;
    while bound < data.len() && matches!(f(&data[bound]), Ordering::Less) {
        bound <<= 1;
    }
    let start = bound >> 1;
    data[start..data.len().min(bound + 1)]
        .binary_search_by(f)
        .map(|i| i + start)
        .map_err(|i| i + start)
}

#[cfg(test)]
mod tests {
    use super::{exp_search, partition};

    #[test]
    fn partition_vector() {
        let mut data = vec![0, 3, 4, 6, 3];
        let split = partition(&mut data, |el| *el != 3);
        assert_eq!(split, 3, "wrong split value");
        assert_eq!(data, vec![0, 4, 6, 3, 3], "wrong partitioned data: {:?}", data);
    }

    #[test]
    fn exponential_search_works_correctly() {
        let data = vec![0, 3, 4, 6, 7];
        let res = exp_search(&data, |el| el.cmp(&0));
        assert_eq!(res, Ok(0), "0");
        let res = exp_search(&data, |el| el.cmp(&3));
        assert_eq!(res, Ok(1), "3");
        let res = exp_search(&data, |el| el.cmp(&5));
        assert_eq!(res, Err(3), "5");
        let res = exp_search(&data, |el| el.cmp(&1000));
        assert_eq!(res, Err(5), "1000");
        let res = exp_search(&data, |el| el.cmp(&-1000));
        assert_eq!(res, Err(0), "-1000");
        let res = exp_search(&data[0..0], |_| panic!("this should not be called"));
        assert_eq!(res, Err(0), "empty");
    }
}
