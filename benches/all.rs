use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};

use hloo::{index::Index, init_lookup};

init_lookup!(LookupUtil, 256, 5, 2, 64);

fn generate_data(n: usize) -> Vec<(Bits, i64)> {
    let mut data = Vec::with_capacity(n);
    for i in 0..n {
        let hash = Bits::new(rand::random());
        data.push((hash, i as i64));
    }
    data
}

fn generate_target(data: &[(Bits, i64)], change_bits: usize) -> Bits {
    let pos = (rand::random::<f32>() * data.len() as f32) as usize;
    let mut target = data[pos].0;
    for _ in 0..change_bits {
        let pos = (rand::random::<f32>() * 31f32) as usize;
        let bit = (target.data[0] & (1 << pos)) >> pos;
        if bit == 0 {
            target.data[0] = target.data[0] | (1 << pos);
        } else {
            target.data[0] = target.data[0] & !(1 << pos);
        }
    }
    target
}

fn index_search_comparison(c: &mut Criterion) {
    println!("preparing data...");
    let data = generate_data(1_000_000);
    let target = generate_target(&data, 3);
    let mut group = c.benchmark_group("single index - search 1M");

    let mut index1 = MemIndex::new(LookupUtil::get(3));
    println!("inserting data into in-memory...");
    index1.insert(&data).unwrap();
    group.bench_function("in-memory", |b| b.iter(|| index1.search(&target, 3)));

    #[cfg(feature = "memmap_index")]
    {
        let tempdir = tempfile::tempdir().unwrap();
        let mut index2 = MemMapIndex::new(LookupUtil::get(3), 0, tempdir.path().join("test-index")).unwrap();
        println!("inserting data into mem-mapped...");
        index2.insert(&data).unwrap();
        group.bench_function("mem-mapped", |b| b.iter(|| index2.search(&target, 3)));
    }

    group.finish();
}

fn search_comparison(c: &mut Criterion) {
    println!("preparing data...");
    let data = generate_data(1_000_000);
    let target = generate_target(&data, 3);
    let mut group = c.benchmark_group("search 1M");

    group.bench_function("naive", |b| b.iter(|| hloo::index::scan_block(&data, &target, 3)));

    let mut lookup1 = LookupUtil::create_mem_lookup::<i64>();
    println!("inserting data into in-memory...");
    lookup1.insert(&data).unwrap();
    group.bench_function("hloo in-memory", |b| b.iter(|| lookup1.search(&target, 3)));

    #[cfg(feature = "memmap_index")]
    {
        let temp_dir = tempfile::tempdir().unwrap();
        println!("inserting data into mem-mapped...");
        let mut lookup2 = LookupUtil::create_memmap_lookup::<i64>(0, temp_dir.path()).unwrap();
        lookup2.insert(&data).unwrap();
        group.bench_function("hloo mem-mapped", |b| b.iter(|| lookup2.search(&target, 3)));
    }

    group.finish();
}

fn insert_comparison(c: &mut Criterion) {
    println!("preparing data...");
    let data = generate_data(100_000);

    let mut group = c.benchmark_group("create + insert 100k");

    group.bench_function("in-memory", |b| {
        b.iter(|| {
            let mut lookup = LookupUtil::create_mem_lookup::<i64>();
            lookup.insert(&data).unwrap();
        })
    });

    #[cfg(feature = "memmap_index")]
    group.bench_function("mem-mapped", |b| {
        b.iter(|| {
            let temp_dir = tempfile::tempdir().unwrap();
            let mut lookup = LookupUtil::create_memmap_lookup::<i64>(0, temp_dir.path()).unwrap();
            lookup.insert(&data).unwrap();
        })
    });

    group.finish();
}

criterion_group!(
    name = index_search;
    config = Criterion::default().sample_size(1000);
    targets = index_search_comparison
);
criterion_group!(
    name = search;
    config = Criterion::default().measurement_time(Duration::from_secs(60)).sample_size(1000);
    targets = search_comparison
);
criterion_group!(
    name = insert;
    config = Criterion::default();
    targets = insert_comparison
);
criterion_main!(index_search, search, insert);
