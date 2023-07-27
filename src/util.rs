/// Merge two sorted vectors into one.
pub fn merge_sorted<T, K>(src1: &[T], src2: &[T], dst: &mut [T], sort_key: impl Fn(T) -> K)
where
    T: Copy,
    K: Ord,
{
    assert_eq!(src1.len() + src2.len(), dst.len(), "buffer is too small");
    let mut idx_left = 0;
    let mut idx_right = 0;
    for el in dst.iter_mut() {
        if idx_left == src1.len() {
            *el = src2[idx_right];
            idx_right += 1;
        } else if idx_right == src2.len() {
            *el = src1[idx_left];
            idx_left += 1;
        } else {
            let left = src1[idx_left];
            let right = src2[idx_right];
            if sort_key(left) < sort_key(right) {
                *el = left;
                idx_left += 1;
            } else {
                *el = right;
                idx_right += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::merge_sorted;

    #[test]
    fn merge_sorted_one_vector() {
        let v1 = vec![1, 4, 5, 5, 5];
        let mut dst = vec![0; v1.len()];
        merge_sorted(&v1, &[], &mut dst, |x| x);
        assert_eq!(v1, dst);

        let v2 = vec![2, 3, 5, 5];
        let mut dst = vec![0; v2.len()];
        merge_sorted(&[], &v2, &mut dst, |x| x);
        assert_eq!(v2, dst);
    }

    #[test]
    fn merge_sorted_two_vectors() {
        let v1 = vec![1, 4, 5, 10, 10];
        let v2 = vec![2, 3, 5, 10];

        let mut dst = vec![0; v1.len() + v2.len()];
        merge_sorted(&v1, &v2, &mut dst, |x| x);

        let mut expected: Vec<_> = v1.into_iter().chain(v2.into_iter()).collect();
        expected.sort();

        assert_eq!(expected, dst)
    }
}
