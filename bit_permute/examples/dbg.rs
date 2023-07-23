use bit_permute::create_permutations;
use itertools::Itertools;

fn main() {
    let total_bits = 256;
    let word_bits = 64;
    let r = 5;
    let k = 2;
    let permutations = create_permutations(total_bits, word_bits, r, k);
    for (i, perm) in permutations.iter().enumerate() {
        println!("\n=== permutation #{} ===", i);
        for block in perm.blocks() {
            println!("{}", block);
        }
        println!("--- compiled apply ---");
        for (word, ops) in perm
            .compile_apply(word_bits, true)
            .iter()
            .sorted_by_key(|i| *i.0)
        {
            println!("w[{}] = {{", word);
            for op in ops {
                println!("  {}", op)
            }
            println!("}}");
        }
    }
    println!("=== compiled mask ===");
    for (_, ops) in permutations[0].compile_top_mask(word_bits, true) {
        for op in ops {
            println!("{}", op)
        }
    }
}
