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

pub use lookup::Lookup;

/// This macro serves as an initialization step to create lookups with specified configuration.
#[macro_export]
macro_rules! init_lookup {
    ($name:ident,$f:literal,$r:literal,$k:literal,$w:literal) => {
        use hloo::bit_permute::{BitPermuter, Distance, DynBitPermuter};
        hloo::make_permutations!(struct_name = "Permutations", f = $f, r = $r, k = $k, w = $w);

        pub struct $name;

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

            pub fn get(i: usize) -> hloo::bit_permute::DynBitPermuter<Bits, Mask> {
                Permutations::get_variant(i)
            }

            pub fn create_mem_lookup<T: Copy + std::fmt::Debug>() -> hloo::Lookup<
                Bits,
                T,
                Mask,
                hloo::bit_permute::DynBitPermuter<Bits, Mask>,
                hloo::index::MemIndex<Bits, T, Mask, hloo::bit_permute::DynBitPermuter<Bits, Mask>>,
            > {
                let permutations = Permutations::get_all_variants();
                let indexes = permutations.into_iter().map(hloo::index::MemIndex::new).collect();
                hloo::Lookup::new(indexes)
            }

            #[cfg(feature = "memmap_index")]
            pub fn create_memmap_lookup<T: Copy + std::fmt::Debug>(
                sig: u64,
                path: &std::path::Path,
            ) -> Result<
                hloo::Lookup<
                    Bits,
                    T,
                    Mask,
                    hloo::bit_permute::DynBitPermuter<Bits, Mask>,
                    hloo::index::MemMapIndex<Bits, T, Mask, hloo::bit_permute::DynBitPermuter<Bits, Mask>>,
                >,
                hloo::index::MemMapIndexError,
            > {
                let sig = Self::signature(sig);
                let mut indexes = Vec::new();
                assert!(path.is_dir(), "path should be a directory!");
                for (i, p) in Permutations::get_all_variants().into_iter().enumerate() {
                    let index_path = path.join(format!("index_{:04}.dat", i));
                    indexes.push(hloo::index::MemMapIndex::new(p, sig, index_path)?)
                }
                Ok(hloo::Lookup::new(indexes))
            }
        }
    };
}
