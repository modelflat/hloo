use criterion::{criterion_group, criterion_main, Criterion};

use hloo::index::BlockLocator;

fn generate_data(n: usize, n_blocks: usize) -> Vec<(usize, usize)> {
    let mut data = Vec::with_capacity(n);
    for i in 0..n {
        data.push((data_gen::rand_between(0, n_blocks as u64) as usize, i));
    }
    data.sort_by_key(|(k, _)| *k);
    data
}

fn generate_targets(data: &[(usize, usize)], n: usize) -> impl Iterator<Item = usize> + '_ {
    let inputs: Vec<usize> = (0..n).map(|_| data_gen::rand_pos(data)).collect();
    inputs.into_iter().map(|i| data[i].0).cycle()
}

fn locate_block_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("locate_block");

    let data_size = 1_000_000;
    let block_counts = [1_000_000, 100_000, 1000, 10];

    for n_blocks in block_counts {
        let data = generate_data(data_size, n_blocks);
        let mut inputs_iter = generate_targets(&data, 100000);

        group.bench_function(format!("bsearch+scan, blocks: {}", n_blocks), |b| {
            b.iter(|| {
                BlockLocator::Naive.locate(&data, &inputs_iter.next().unwrap(), |x| *x);
            })
        });

        group.bench_function(format!("double bsearch, blocks: {}", n_blocks), |b| {
            b.iter(|| {
                BlockLocator::DoubleBsearch.locate(&data, &inputs_iter.next().unwrap(), |x| *x);
            })
        });
    }

    group.finish();
}

criterion_group!(
    name = locate_block;
    config = Criterion::default().sample_size(1000);
    targets = locate_block_bench
);
criterion_main!(locate_block);
