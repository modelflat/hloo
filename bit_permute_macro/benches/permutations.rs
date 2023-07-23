use criterion::{criterion_group, criterion_main, Criterion};

use bit_permute_macro::make_permutations;

make_permutations!(struct_name = "Permutation", f = 256, r = 5, k = 2);

fn generate_data(n: usize) -> Vec<Bits> {
    let mut data = Vec::with_capacity(n);
    for _ in 0..n {
        data.push(Bits::new([
            rand::random(),
            rand::random(),
            rand::random(),
            rand::random(),
        ]));
    }
    data
}

fn apply_bench(c: &mut Criterion) {
    let data = generate_data(1 << 10);
    for i in 0..10 {
        let permutation = PermutationUtil::get_variant(i);

        c.bench_function(&format!("permutation.apply {}", i), |b| {
            b.iter(|| permutation.apply(data[0]))
        });
    }
}

fn mask_bench(c: &mut Criterion) {
    let data = generate_data(1 << 10);

    for i in 0..10 {
        let permutation = PermutationUtil::get_variant(i);

        c.bench_function(&format!("permutation.mask {}", i), |b| {
            b.iter(|| permutation.mask(&data[0]))
        });
    }
}

criterion_group!(benches, apply_bench, mask_bench);
criterion_main!(benches);
