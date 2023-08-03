use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};

use data_gen::{flip_bits, generate_uniform_data, generate_uniform_data_with_block_size, rand_pos};
use hloo::index::naive_search;

hloo::init_lookup!(LookupUtil, 256, 5, 1, 64);

#[allow(unused)]
fn generate_perfect_data(n: usize, _: usize) -> Vec<(Bits, usize)> {
    generate_uniform_data(n).map(|(k, v)| (Bits::new(k), v)).collect()
}

#[allow(unused)]
fn generate_bad_data(n: usize, block_size: usize) -> Vec<(Bits, usize)> {
    generate_uniform_data_with_block_size(n, block_size, false, Bits::new).collect()
}

fn generate_target(data: &[(Bits, usize)], change_bits: usize) -> Bits {
    let pos = rand_pos(data);
    Bits::new(flip_bits(data[pos].0.data, change_bits))
}

fn search_bench(c: &mut Criterion) {
    println!("preparing data...");
    let data = generate_perfect_data(1_000_000, 10);
    let target = generate_target(&data, 3);
    let mut group = c.benchmark_group("search 1M");

    group.bench_function("naive", |b| b.iter(|| naive_search(&data, target, 3)));

    let mut lookup1 = LookupUtil::create_mem_lookup();
    println!("inserting data into in-memory...");
    lookup1.insert(&data).unwrap();
    group.bench_function("hloo in-memory", |b| b.iter(|| lookup1.search(&target, 3)));

    let temp_dir = tempfile::tempdir().unwrap();
    println!("inserting data into mem-mapped...");
    let mut lookup2 = LookupUtil::create_memmap_lookup(0, temp_dir.path()).unwrap();
    lookup2.insert(&data).unwrap();
    group.bench_function("hloo mem-mapped", |b| b.iter(|| lookup2.search(&target, 3)));

    group.finish();
}

criterion_group!(
    name = search;
    config = Criterion::default().measurement_time(Duration::from_secs(60)).sample_size(1000);
    targets = search_bench
);
criterion_main!(search);
