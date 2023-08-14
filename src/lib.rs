//! Basic usage:
//!
//! ```
//! // 1) Create a Lookup Util (sort of a factory for lookups)
//! hloo::init_lookup!(LookupUtil, 32, 5, 1, 32);
//! // 2) Create lookup with the types you need from permuter
//! let mut lookup = LookupUtil::create_mem_lookup::<i64>();
//! // 3) Use your lookup
//! lookup.insert(&[(Bits::default(), 123456)]);
//! lookup.search(&Bits::default(), 4);
//! ```
//!
//! Alternatively, you can use one of the pre-defined implementations:
//!
//! ```
//! use hloo::lookup::lookup_impl::{lookup64, lookup128, lookup192, lookup256};
//!
//! // in-memory
//! let mem_lookup = lookup64::MemLookup::<i64>::new();
//!
//! // memory-mapped
//! let path: std::path::PathBuf = "/tmp/some-path".try_into().unwrap();
//! let memmap_lookup = lookup64::MemMapLookup::<i64>::create(&path);
//! ```

pub mod index;
pub mod lookup;
pub mod util;

pub mod mmvec;

use std::sync::Arc;

pub use hloo_core;
pub use hloo_macros::make_permutations;

pub use index::Index;
pub use lookup::{Lookup, SimpleLookup};

pub type DynIndex<K, V, M, E> = Arc<dyn Index<K, V, M, Error = E>>;

pub type DynBitPermuter<B, M> = Box<dyn hloo_core::BitPermuter<Bits = B, Mask = M>>;

/// This macro serves as an initialization step to create lookups with specified configuration.
#[macro_export]
macro_rules! init_lookup {
    ($name:ident,$f:literal,$r:literal,$k:literal,$w:literal) => {
        use hloo::{
            hloo_core::{BitIndex, BitPermuter, Distance},
            Lookup,
        };
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

        pub type MemIndex<T> = hloo::index::MemIndex<Bits, T, Mask>;
        pub type MemLookup<T> = hloo::SimpleLookup<Bits, T, Mask, MemIndex<T>>;
        pub type MemMapIndex<T> = hloo::index::MemMapIndex<Bits, T, Mask>;
        pub type MemMapLookup<T> = hloo::SimpleLookup<Bits, T, Mask, MemMapIndex<T>>;

        impl $name {
            pub fn create_mem_lookup<T>() -> MemLookup<T> {
                let permutations = Permutations::get_all_variants();
                let indexes = permutations.into_iter().map(MemIndex::new).collect();
                MemLookup::new(indexes)
            }

            pub fn create_memmap_lookup<T: Copy + 'static>(
                path: &std::path::Path,
            ) -> Result<MemMapLookup<T>, hloo::index::MemMapIndexError> {
                let sig = hloo::util::sign_type::<T>($f, $r, $k, $w);
                Ok(MemMapLookup::create(Permutations::get_all_variants(), sig, path)?)
            }

            pub fn load_memmap_lookup<T: Copy + 'static>(
                path: &std::path::Path,
            ) -> Result<MemMapLookup<T>, hloo::index::MemMapIndexError> {
                let sig = hloo::util::sign_type::<T>($f, $r, $k, $w);
                Ok(MemMapLookup::load(Permutations::get_all_variants(), sig, path)?)
            }
        }
    };
}
