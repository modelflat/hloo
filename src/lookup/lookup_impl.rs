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
                util::sign_type,
                SimpleLookup,
            };

            pub use internal::{Bits, Mask, Permutations};

            mod internal {
                use hloo_core::{BitContainer, BitPermuter};
                crate::make_permutations!(struct_name = "Permutations", f = $f, r = $r, k = $k, w = $w);
            }

            impl_lookup!(MemLookup, MemIndex);

            impl<V> Default for MemLookup<V>
            where
                V: Copy,
            {
                fn default() -> Self {
                    let perms = Permutations::get_all_variants();
                    Self(SimpleLookup::new(perms.into_iter().map(MemIndex::new).collect()))
                }
            }

            impl_lookup!(MemMapLookup, MemMapIndex);
            impl<V> MemMapLookup<V>
            where
                V: Copy + 'static,
            {
                pub fn create(
                    path: &std::path::Path,
                ) -> Result<Self, <MemMapIndex<Bits, V, Mask> as PersistentIndex<Bits, Mask>>::Error> {
                    let sig = sign_type::<V>($f, $r, $k, $w);
                    Ok(Self(SimpleLookup::create(
                        Permutations::get_all_variants(),
                        sig,
                        path,
                    )?))
                }

                pub fn load(
                    path: &std::path::Path,
                ) -> Result<Self, <MemMapIndex<Bits, V, Mask> as PersistentIndex<Bits, Mask>>::Error> {
                    let sig = sign_type::<V>($f, $r, $k, $w);
                    Ok(Self(SimpleLookup::load(
                        Permutations::get_all_variants(),
                        sig,
                        path,
                    )?))
                }
            }
        }
    };
}

impl_lookups!(lookup64, 64, 4, 1, 64);
impl_lookups!(lookup256, 256, 8, 1, 64);

pub enum DynBits {
    Bits64(lookup64::Bits),
    Bits256(lookup256::Bits),
}

impl From<&[u8]> for DynBits {
    fn from(value: &[u8]) -> Self {
        match value.len() {
            lookup64::Bits::SIZE_BYTES => Self::Bits64(lookup64::Bits::from_le_bytes(value)),
            lookup256::Bits::SIZE_BYTES => Self::Bits256(lookup256::Bits::from_le_bytes(value)),
            len => panic!("invalid slice size: {len}"),
        }
    }
}

pub enum DynBitsVec {
    Bits64(lookup64::Bits),
}
