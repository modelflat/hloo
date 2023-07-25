//! Basic usage:
//!
//! ```
//! // 1) Create a permuter using create_permuter! macro
//! hloo::create_permuter!(Permuter, 32, 5, 2, 32);
//! // 2) Create lookup with the types you need from permuter
//! let mut lookup = Permuter::create_in_memory_lookup::<i64>();
//! // 3) Use your lookup
//! lookup.insert(&[(Bits::new(rand::random()), 123456)]);
//! lookup.search(Bits::new(rand::random()), 5);
//! ```

mod index;
mod lookup;
mod mmvec;
mod util;

pub use bit_permute_macro::make_permutations;

pub use index::{memmap_index::MemMapIndex, memory_index::MemoryIndex, BitPermuter, Index};
pub use lookup::Lookup;
pub use mmvec::MmVec;

#[macro_export]
macro_rules! create_permuter {
    ($name:ident,$f:literal,$r:literal,$k:literal,$w:literal) => {
        hloo::make_permutations!(struct_name = "Permutation", f = $f, r = $r, k = $k, w = $w);
        pub struct $name(std::sync::Arc<dyn Permutation>);
        impl $name {
            pub fn all() -> Vec<Self> {
                PermutationUtil::get_all_variants()
                    .into_iter()
                    .map(Self)
                    .collect()
            }
            pub fn get(i: usize) -> Self {
                Self(PermutationUtil::get_variant(i))
            }
            pub fn create_in_memory_lookup<T: Copy>(
            ) -> hloo::Lookup<Bits, T, Mask, Self, hloo::MemoryIndex<Bits, T, Mask, Self>> {
                let indexes = Self::all().into_iter().map(hloo::MemoryIndex::new).collect();
                hloo::Lookup::new(indexes)
            }
            pub fn create_mem_map_lookup<T: Copy>(
                path: &std::path::Path,
            ) -> std::io::Result<hloo::Lookup<Bits, T, Mask, Self, hloo::MemMapIndex<Bits, T, Mask, Self>>> {
                let mut indexes = Vec::new();
                assert!(path.is_dir(), "path should be a directory!");
                for (i, p) in Self::all().into_iter().enumerate() {
                    let index_path = path.join(format!("index_{:04}.dat", i));
                    indexes.push(hloo::MemMapIndex::new(p, index_path)?)
                }
                Ok(hloo::Lookup::new(indexes))
            }
        }
        impl hloo::BitPermuter<Bits, Mask> for $name {
            fn apply(&self, key: Bits) -> Bits {
                self.0.apply(key)
            }
            fn mask(&self, key: &Bits) -> Mask {
                self.0.mask(key)
            }
            fn dist(&self, key1: &Bits, key2: &Bits) -> u32 {
                key1.xor_count_ones(key2)
            }
        }
    };
}
