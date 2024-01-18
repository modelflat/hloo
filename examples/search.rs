use std::time::Instant;

use data_gen::{flip_bits, generate_uniform_data, rand_pos};
use hloo::{
    lookup::lookup_impl::lookup256::{Bits, MemLookup},
    Lookup,
};

fn generate_perfect_data(n: usize, _: usize) -> Vec<(Bits, usize)> {
    generate_uniform_data(n).map(|(k, v)| (Bits::new(k), v)).collect()
}

fn generate_target(data: &[(Bits, usize)], change_bits: usize) -> Bits {
    let pos = rand_pos(data);
    Bits::new(flip_bits(data[pos].0.data, change_bits))
}

fn main() {
    println!("preparing data...");
    let data = generate_perfect_data(1_000_000, 10);

    let mut lookup = MemLookup::default();
    println!("inserting data into in-memory...");
    lookup.insert(&data).unwrap();

    println!("running search...");
    let t = Instant::now();
    let mut side_effect = 0;
    for _ in 0..10000 {
        let target = generate_target(&data, 3);
        for _ in 0..1000 {
            side_effect += lookup.search(&target, 3).map_or(0, |r| r.candidates_scanned);
        }
    }
    let t = Instant::now() - t;
    println!("total time taken: {} ms", t.as_millis());
    println!("total candidates scanned: {}", side_effect);
}
