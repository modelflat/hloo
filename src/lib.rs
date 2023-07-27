//! Basic usage:
//!
//! ```
//! // 1) Create a permuter using create_permuter! macro
//! hloo::create_permuter!(Permuter, 32, 5, 1, 32);
//! // 2) Create lookup with the types you need from permuter
//! let mut lookup = Permuter::create_mem_lookup::<i64>();
//! // 3) Use your lookup
//! lookup.insert(&[(Bits::new(rand::random()), 123456)]);
//! lookup.search(Bits::new(rand::random()), 4);
//! ```

pub mod index;
pub mod lookup;
pub mod mmvec;
pub mod util;

pub use bit_permute_macro::make_permutations;

pub use lookup::Lookup;

#[macro_export]
macro_rules! create_permuter {
    ($name:ident,$f:literal,$r:literal,$k:literal,$w:literal) => {
        hloo::make_permutations!(struct_name = "Permutation", f = $f, r = $r, k = $k, w = $w);
        pub struct $name(std::sync::Arc<dyn Permutation>);
        impl $name {
            pub fn get(i: usize) -> Self {
                Self(PermutationUtil::get_variant(i))
            }
            pub fn create_mem_lookup<T: Copy + std::fmt::Debug>(
            ) -> hloo::Lookup<Bits, T, Mask, Self, hloo::index::MemIndex<Bits, T, Mask, Self>> {
                let permutations = PermutationUtil::get_all_variants();
                let indexes = permutations
                    .into_iter()
                    .map(Self)
                    .map(hloo::index::MemIndex::new)
                    .collect();
                hloo::Lookup::new(indexes)
            }
            pub fn create_memmap_lookup<T: Copy + std::fmt::Debug>(
                path: &std::path::Path,
            ) -> Result<
                hloo::Lookup<Bits, T, Mask, Self, hloo::index::MemMapIndex<Bits, T, Mask, Self>>,
                hloo::index::MemMapIndexError,
            > {
                let permutations = PermutationUtil::get_all_variants();
                let mut indexes = Vec::new();
                assert!(path.is_dir(), "path should be a directory!");
                for (i, p) in permutations.into_iter().enumerate() {
                    let index_path = path.join(format!("index_{:04}.dat", i));
                    indexes.push(hloo::index::MemMapIndex::new(Self(p), index_path)?)
                }
                Ok(hloo::Lookup::new(indexes))
            }
        }
        impl hloo::index::BitPermuter<Bits, Mask> for $name {
            fn apply(&self, key: Bits) -> Bits {
                self.0.apply(key)
            }
            fn mask(&self, key: &Bits) -> Mask {
                self.0.mask(key)
            }
            fn dist(&self, key1: &Bits, key2: &Bits) -> u32 {
                key1.xor_count_ones(key2)
            }
            fn n_blocks() -> u32 {
                PermutationUtil::n_blocks() as u32
            }
        }
    };
}
