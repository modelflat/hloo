use std::path::Path;

use criterion::{criterion_group, criterion_main, Criterion};

use hloo::MmVec;
use tempfile::TempDir;

use hloo::make_permutations;

// fn generate_data(n: usize) -> Vec<HashTuple> {
//     let mut data = Vec::with_capacity(n);
//     for i in 0..n {
//         let hash = HashValue::new(rand::random());
//         data.push((i as ImageId, hash));
//     }
//     data
// }

// fn generate_data_mmap(n: usize, path: &Path) -> MmVec<HashTuple> {
//     let gen = generate_data(n);
//     MmVec::from_vec(gen, path.to_path_buf()).expect("failed to create mmaped vec")
// }

fn table_search_bench(c: &mut Criterion) {
    println!("preparing data...");
    let tmp_dir = TempDir::new().expect("failed to create index temp dir");
    // let data = generate_data_mmap(100_000_000, &tmp_dir.path().join("table.mmap"));
    // let permutation = PermutationUtil::get_variant(3);

    todo!()

    // let table = SearchTable::new(permutation, data);

    // c.bench_function("table.search", |b| {
    //     b.iter(|| {
    //         let target = HashValue::new(rand::random());
    //         table.search(target, 6)
    //     })
    // });
}

fn index_search_bench(c: &mut Criterion) {
    println!("preparing data...");
    let tmp_dir = TempDir::new().expect("failed to create index temp dir");
    // let data = generate_data_mmap(100_000_000, &tmp_dir.path().join("index.data.mmap"));

    todo!()

    // let index = Index::new(data, tmp_dir.path()).expect("failed to create index");

    // c.bench_function("index.search", |b| {
    //     b.iter(|| {
    //         let target = HashValue::new(rand::random());
    //         index.search_v2(target, 6)
    //     })
    // });
}

criterion_group!(benches, table_search_bench, index_search_bench);
criterion_main!(benches);
