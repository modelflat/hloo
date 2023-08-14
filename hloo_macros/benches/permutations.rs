use criterion::{criterion_group, criterion_main, Criterion};
use rand::random;

use hloo_core::{BitContainer, BitPermuter};
use hloo_macros::make_permutations;

make_permutations!(struct_name = "Permutations", f = 256, r = 5, k = 2, w = 64);

fn apply_bench(c: &mut Criterion) {
    let data = Bits::new(random());
    let mut group = c.benchmark_group("apply");
    for i in 0..10 {
        let permutation = Permutations::get_variant(i);
        group.bench_function(&format!("{}", i), |b| b.iter(|| permutation.apply(&data)));
    }
    group.finish();
}

fn mask_bench(c: &mut Criterion) {
    let data = Bits::new(random());
    let mut group = c.benchmark_group("mask");
    for i in 0..10 {
        let permutation = Permutations::get_variant(i);
        group.bench_function(&format!("{}", i), |b| b.iter(|| permutation.mask(&data)));
    }
    group.finish();
}

criterion_group!(benches, apply_bench, mask_bench);
criterion_main!(benches);
