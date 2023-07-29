//! Basic usage:
//!
//! ```
//! // 1) Create a Lookup Util (sort of a factory for lookups)
//! hloo::init_lookup!(LookupUtil, 32, 5, 1, 32);
//! // 2) Create lookup with the types you need from permuter
//! let mut lookup = LookupUtil::create_mem_lookup::<i64>();
//! // 3) Use your lookup
//! lookup.insert(&[(Bits::new(rand::random()), 123456)]);
//! lookup.search(&Bits::new(rand::random()), 4);
//! ```

pub mod index;
pub mod lookup;
pub mod util;

#[cfg(feature = "memmap_index")]
pub mod mmvec;

pub use bit_permute;
pub use bit_permute_macro::make_permutations;

pub use index::Index;
pub use lookup::Lookup;

/// This macro serves as an initialization step to create lookups with specified configuration.
#[macro_export]
macro_rules! init_lookup {
    ($name:ident,$f:literal,$r:literal,$k:literal,$w:literal) => {
        use hloo::bit_permute::{BitPermuter, Distance, DynBitPermuter};
        hloo::make_permutations!(struct_name = "Permutations", f = $f, r = $r, k = $k, w = $w);

        #[doc = "This struct can create or load lookups with the following underlying "]
        #[doc = "bit permutation parameters: f = "]
        #[doc = stringify!($f)]
        #[doc = ", r = "]
        #[doc = stringify!($r)]
        #[doc = ", k = "]
        #[doc = stringify!($k)]
        #[doc = ", w = "]
        #[doc = stringify!($w)]
        pub struct $name;

        pub type Permuter = DynBitPermuter<Bits, Mask>;

        pub type MemIndex<T> = hloo::index::MemIndex<Bits, T, Mask, Permuter>;
        pub type MemLookup<T> = hloo::Lookup<Bits, T, Mask, Permuter, MemIndex<T>>;

        #[cfg(feature = "memmap_index")]
        pub type MemMapIndex<T> = hloo::index::MemMapIndex<Bits, T, Mask, Permuter>;
        #[cfg(feature = "memmap_index")]
        pub type MemMapLookup<T> = hloo::Lookup<Bits, T, Mask, Permuter, MemMapIndex<T>>;

        impl $name {
            pub fn signature(type_sig: u64) -> u64 {
                use std::{collections::hash_map::DefaultHasher, hash::Hasher};
                let mut hasher = DefaultHasher::new();
                hasher.write_u64($f);
                hasher.write_u64($r);
                hasher.write_u64($k);
                hasher.write_u64($w);
                hasher.write_u64(type_sig);
                hasher.finish()
            }

            pub fn create_mem_lookup<T>() -> MemLookup<T> {
                let permutations = Permutations::get_all_variants();
                let indexes = permutations.into_iter().map(MemIndex::new).collect();
                MemLookup::new(indexes)
            }

            #[cfg(feature = "memmap_index")]
            pub fn create_memmap_lookup<T: Copy>(
                sig: u64,
                path: &std::path::Path,
            ) -> Result<MemMapLookup<T>, hloo::index::MemMapIndexError> {
                let sig = Self::signature(sig);
                Ok(MemMapLookup::create(Permutations::get_all_variants(), sig, path)?)
            }

            #[cfg(feature = "memmap_index")]
            pub fn load_memmap_lookup<T: Copy>(
                sig: u64,
                path: &std::path::Path,
            ) -> Result<MemMapLookup<T>, hloo::index::MemMapIndexError> {
                let sig = Self::signature(sig);
                Ok(MemMapLookup::load(Permutations::get_all_variants(), sig, path)?)
            }
        }
    };
}
