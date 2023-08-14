macro_rules! impl_lookup {
    ($name:ident,$index:ident) => {
        pub struct $name<V: Copy>(
            SimpleLookup<internal::Bits, V, internal::Mask, $index<internal::Bits, V, internal::Mask>>,
        );

        impl<V> Lookup<internal::Bits, V, internal::Mask> for $name<V>
        where
            V: Copy,
        {
            type Index = $index<internal::Bits, V, internal::Mask>;

            fn indexes(&self) -> &[Self::Index] {
                self.0.indexes()
            }

            fn indexes_mut(&mut self) -> &mut [Self::Index] {
                self.0.indexes_mut()
            }
        }
    };
}

macro_rules! impl_lookups {
    ($mod_name:ident,$f:literal,$r:literal,$k:literal,$w:literal) => {
        pub mod $mod_name {
            use crate::{
                index::{MemIndex, MemMapIndex, PersistentIndex},
                lookup::Lookup,
                SimpleLookup,
            };

            pub use internal::{Bits, Mask, Permutations};

            mod internal {
                use hloo_core::{BitIndex, BitPermuter, Distance};
                crate::make_permutations!(struct_name = "Permutations", f = $f, r = $r, k = $k, w = $w);
            }

            impl_lookup!(MemLookup, MemIndex);
            impl<V> MemLookup<V>
            where
                V: Copy,
            {
                pub fn new() -> Self {
                    let perms = Permutations::get_all_variants();
                    Self(SimpleLookup::new(perms.into_iter().map(MemIndex::new).collect()))
                }
            }

            impl_lookup!(MemMapLookup, MemMapIndex);
            impl<V> MemMapLookup<V>
            where
                V: Copy,
            {
                pub fn create(
                    sig: u64,
                    path: &std::path::Path,
                ) -> Result<Self, <MemMapIndex<Bits, V, Mask> as PersistentIndex<Bits, Mask>>::Error> {
                    let perms = Permutations::get_all_variants();
                    Ok(Self(SimpleLookup::new(
                        perms
                            .into_iter()
                            .map(|p| MemMapIndex::create(p, sig, path))
                            .collect::<Result<_, _>>()?,
                    )))
                }
            }
        }
    };
}

impl_lookups!(lookup64, 64, 4, 1, 64);
impl_lookups!(lookup128, 128, 5, 1, 64);
impl_lookups!(lookup192, 192, 6, 1, 64);
impl_lookups!(lookup256, 256, 8, 1, 64);
