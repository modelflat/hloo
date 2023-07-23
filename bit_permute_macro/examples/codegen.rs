use bit_permute_macro::make_permutations;

make_permutations!(struct_name = "Permutation", f = 256, r = 5, k = 1, w = 64);

fn main() {
    let perm = PermutationUtil::get_variant(0);
    let val: Bits = Default::default();
    let mask = perm.mask(&val);
    println!("mask = {:?}", mask);
}
