use criterion::{criterion_group, criterion_main, Criterion};

use hloo::index::select_block_at;

fn random(l: usize, h: usize) -> usize {
    let f: f32 = rand::random();
    (l as f32 + f * (h - l) as f32) as usize
}

fn generate_data(n: usize, n_blocks: usize) -> Vec<(usize, usize)> {
    let mut data = Vec::with_capacity(n);
    for i in 0..n {
        data.push((random(0, n_blocks), i));
    }
    data.sort_by_key(|(k, _)| *k);
    data
}

fn select_block_at_bench(c: &mut Criterion) {
    println!("preparing data...");
    let data = generate_data(100000, 1000);
    let inputs: Vec<usize> = (0..10000).map(|_| random(0, data.len())).collect();
    let mut inputs_iter = inputs.into_iter().cycle();

    c.bench_function("search_block_at", |b| {
        b.iter(|| {
            select_block_at(&data, inputs_iter.next().unwrap(), |x| *x);
        })
    });
}

criterion_group!(
    name = select_block;
    config = Criterion::default().sample_size(1000);
    targets = select_block_at_bench
);
criterion_main!(select_block);
