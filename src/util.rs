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

#[cfg(test)]
mod tests {
    use super::partition;

    #[test]
    fn partition_vector() {
        let mut data = vec![0, 3, 4, 6, 3];
        let split = partition(&mut data, |el| *el != 3);
        assert_eq!(split, 3, "wrong split value");
        assert_eq!(data, vec![0, 4, 6, 3, 3], "wrong partitioned data: {:?}", data);
    }
}
