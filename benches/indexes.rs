use criterion::{criterion_group, criterion_main, Criterion};

use data_gen::{flip_bits, generate_uniform_data, generate_uniform_data_with_block_size, rand_pos};
use hloo::{index::Index, init_lookup};

init_lookup!(LookupUtil, 256, 5, 1, 64);

#[allow(unused)]
fn generate_perfect_data(n: usize, _: usize) -> Vec<(Bits, usize)> {
    generate_uniform_data(n).map(|(k, v)| (Bits::new(k), v)).collect()
}

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

fn index_search_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("single index - search 1M");

    let data_size = 1_000_000;
    let block_sizes = [10, 1000, 100000];
    let permutations = Permutations::get_all_variants();

    for block_size in block_sizes {
        let data = generate_perfect_data(data_size, block_size);
        let mut target_iter = generate_targets(&data, data_size, 3);
        for (i, perm) in permutations.iter().enumerate() {
            let mut index = MemIndex::new(perm.clone());
            index.insert(&data).unwrap();
            index.refresh();
            println!("{:?}", index.stats());
            group.bench_function(
                format!("in-memory, perm_i = {}, avg block_size = {}", i, block_size),
                |b| b.iter(|| index.search(&target_iter.next().unwrap(), 3)),
            );
        }
    }

    for block_size in block_sizes {
        let data = generate_perfect_data(data_size, block_size);
        let mut target_iter = generate_targets(&data, data_size, 3);
        for (i, perm) in permutations.iter().enumerate() {
            let tempdir = tempfile::tempdir().unwrap();
            let mut index = MemMapIndex::new(perm.clone(), 0, tempdir.path().join("test-index")).unwrap();
            index.insert(&data).unwrap();
            index.refresh();
            println!("{:?}", index.stats());
            group.bench_function(
                format!("mem-mapped, perm_i = {}, avg block_size = {}", i, block_size),
                |b| b.iter(|| index.search(&target_iter.next().unwrap(), 3)),
            );
        }
    }

    group.finish();
}

criterion_group!(
    name = index_search;
    config = Criterion::default();
    targets = index_search_comparison
);
criterion_main!(index_search);
