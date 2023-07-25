use criterion::{criterion_group, criterion_main, Criterion};

use hloo::{create_permuter, Index, MemMapIndex, MemoryIndex};

create_permuter!(MyPermuter, 32, 5, 2, 32);

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

fn memory_index_search_bench(c: &mut Criterion) {
    let permutation = MyPermuter::get(3);
    let mut index = MemoryIndex::new(permutation);
    println!("preparing data...");
    let data = generate_data(100_000_000);
    println!("inserting data into index...");
    index.insert(&data).unwrap();

    let target = generate_target(&data, 5);
    c.bench_function("memory index - search", |b| b.iter(|| index.search(target, 6)));
}

fn memvec_index_search_bench(c: &mut Criterion) {
    let tempdir = tempfile::tempdir().expect("failed to create temp dir");
    let path = tempdir.path().join("index.bin");
    let permutation = MyPermuter::get(3);
    let mut index = MemMapIndex::new(permutation, path).expect("failed to create index");
    println!("preparing data...");
    let data = generate_data(100_000_000);
    println!("inserting data into index...");
    index.insert(&data).expect("failed to insert data to index");

    let target = generate_target(&data, 5);
    c.bench_function("memory index - search", |b| b.iter(|| index.search(target, 6)));
}

criterion_group!(benches, memory_index_search_bench, memvec_index_search_bench);
criterion_main!(benches);
