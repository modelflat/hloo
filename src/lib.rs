//! TODO more ergonomic interfaces
//!
//! Basic usage:
//!
//! ```
//! // 1) Create a permuter using create_permuter! macro
//! hloo::create_permuter!(MyPermuter, 32, 5, 2, 32);
//! // 2) Create your indexes using your permuter
//! let mut index = hloo::MemoryIndex::new(MyPermuter::get(3));
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
        }
        impl hloo::BitPermuter<Bits, i64, Mask> for $name {
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
