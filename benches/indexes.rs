use criterion::{criterion_group, criterion_main, Criterion};

use data_gen::{flip_bits, generate_uniform_data_with_block_size, rand_pos};
use hloo::{index::Index, init_lookup};

init_lookup!(LookupUtil, 256, 5, 1, 64);

#[allow(unused)]
fn generate_bad_data(n: usize, block_size: usize) -> Vec<(Bits, usize)> {
    generate_uniform_data_with_block_size(n, block_size, false, Bits::new).collect()
}

fn generate_targets(data: &[(Bits, usize)], n: usize, change_bits: usize) -> impl Iterator<Item = Bits> {
    let src = (0..n)
        .map(|_| Bits::new(flip_bits(data[rand_pos(data)].0.data, change_bits)))
        .collect::<Vec<_>>();
    src.into_iter().cycle()
}

fn index_get_candidates_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_candidates 1M");

    let data_size = 1_000_000;
    let block_sizes = [10, 1000, 100000];

    for block_size in block_sizes {
        let data = generate_bad_data(data_size, block_size);
        let mut target_iter = generate_targets(&data, data_size, 3);
        let mut index = MemIndex::new(Permutations::get_variant(0));
        index.insert(&data).unwrap();
        index.refresh();
        println!("{:?}", index.stats());
        group.bench_function(format!("in-memory, perm_0, avg block_size = {}", block_size), |b| {
            b.iter(|| index.get_candidates(&target_iter.next().unwrap()))
        });
    }

    for block_size in block_sizes {
        let data = generate_bad_data(data_size, block_size);
        let mut target_iter = generate_targets(&data, data_size, 3);
        let tempdir = tempfile::tempdir().unwrap();
        let mut index = MemMapIndex::new(Permutations::get_variant(0), 0, tempdir.path().join("test-index")).unwrap();
        index.insert(&data).unwrap();
        index.refresh();
        println!("{:?}", index.stats());
        group.bench_function(format!("mem-mapped, perm_0, avg block_size = {}", block_size), |b| {
            b.iter(|| index.get_candidates(&target_iter.next().unwrap()))
        });
    }

    group.finish();
}

criterion_group!(
    name = index_get_candidates;
    config = Criterion::default();
    targets = index_get_candidates_bench
);
criterion_main!(index_get_candidates);
