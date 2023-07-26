pub fn merge_sorted_alloc<T, K>(src1: &[T], src2: &[T], sort_key: impl Fn(T) -> K) -> Vec<T>
where
    T: Copy + Default,
    K: Ord,
{
    let len = src1.len() + src2.len();
    let mut dst = Vec::with_capacity(len);
    // SAFETY: merge_sorted does not read from `dst`
    #[allow(clippy::uninit_vec)]
    unsafe {
        dst.set_len(len)
    };
    merge_sorted(src1, src2, &mut dst, sort_key);
    dst
}

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
            if sort_key(left) <= sort_key(right) {
                *el = left;
                idx_left += 1;
            } else {
                *el = right;
                idx_right += 1;
            }
        }
    }
}
