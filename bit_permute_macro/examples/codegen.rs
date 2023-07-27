use bit_permute::{BitPermuter, Distance, DynBitPermuter};
use bit_permute_macro::make_permutations;

make_permutations!(struct_name = "Permutations", f = 256, r = 5, k = 1, w = 64);

fn main() {}
