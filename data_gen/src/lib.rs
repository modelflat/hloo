use itertools::Itertools;

pub use rand::random;

pub fn rand_between(l: u64, h: u64) -> u64 {
    assert!(l <= h, "l should be <= h");
    let r: f32 = rand::random();
    l + (r * (h - l) as f32) as u64
}

pub fn rand_pos<T>(data: &[T]) -> usize {
    (rand::random::<f32>() * data.len() as f32) as usize
}

pub fn generate_uniform_data(n: usize) -> impl Iterator<Item = ([u64; 4], usize)> {
    (0..n).map(|i| (rand::random(), i)).sorted_unstable_by_key(|(k, _)| *k)
}

pub fn generate_uniform_data_with_block_size<T>(
    n: usize,
    block_size: usize,
    really_bad_distribution: bool,
    map_to: impl Fn([u64; 4]) -> T,
) -> impl Iterator<Item = (T, usize)> {
    generate_uniform_data(n)
        .map(move |mut el| {
            el.0[0] = rand_between(0, (n / block_size) as u64) << 32;
            if really_bad_distribution {
                el.0[1] = el.0[0];
                el.0[2] = el.0[0];
                el.0[3] = el.0[0];
            }
            el
        })
        .sorted_unstable_by_key(|(k, _)| *k)
        .map(move |(k, v)| (map_to(k), v))
}

pub fn flip_bits<const S: usize>(mut bits: [u64; S], n: usize) -> [u64; S] {
    for _ in 0..n {
        let pos = (rand::random::<f32>() * 31f32) as usize;
        let bit = (bits[0] & (1 << pos)) >> pos;
        if bit == 0 {
            bits[0] |= 1 << pos;
        } else {
            bits[0] &= !(1 << pos);
        }
    }
    bits
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::generate_uniform_data_with_block_size;

    #[test]
    fn generate_block_sizes_works() {
        let res: Vec<_> = generate_uniform_data_with_block_size(100000, 1000, false, |x| x)
            .map(|x| x.0[0])
            .dedup()
            .collect();
        assert!(
            res.len() <= 100,
            "should be no more than 100 distinct masks, got {}",
            res.len()
        )
    }
}
