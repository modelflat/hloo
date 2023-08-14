use hloo_core::{BitContainer, BitPermuter};
use hloo_macros::make_permutations;

make_permutations!(struct_name = "Permutations", f = 256, r = 5, k = 1, w = 64);

fn main() {}
