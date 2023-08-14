use darling::{
    export::syn::{Type, TypeArray},
    FromMeta, ToTokens,
};
use proc_macro2::Ident;
use quote::{format_ident, quote};

pub struct Bits<'a> {
    type_name: &'a Ident,
    word_type_name: &'a Ident,
    word_size: usize,
    n_words: usize,
}

impl<'a> Bits<'a> {
    pub fn new(type_name: &'a Ident, word_type_name: &'a Ident, word_size: usize, n_words: usize) -> Self {
        Self {
            type_name,
            word_type_name,
            word_size,
            n_words,
        }
    }
}

impl ToTokens for Bits<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let type_name = self.type_name;
        let storage_type_name = format_ident!("{}Data", type_name);
        let word_type_name = self.word_type_name;
        let iterator_name = format_ident!("{}Iterator", type_name);
        let full_size = self.word_size * self.n_words;
        let byte_size = full_size / 8;
        let word_size = self.word_size;
        let word_bytes = self.word_size / 8;
        let word_range_le = 0..self.n_words;
        let word_range_be = word_range_le.clone();
        let word_range_xor = word_range_le.clone();
        let word_max = word_range_le.clone().map(|_| word_type_name.clone());

        let data_type = match TypeArray::from_string(&format!("[{}; {}]", self.word_type_name, self.n_words)) {
            Ok(arr) => Type::Array(arr),
            Err(e) => {
                tokens.extend(e.write_errors());
                return;
            }
        };

        let code = quote! {
            pub type #storage_type_name = #data_type;

            #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
            #[repr(C)]
            pub struct #type_name {
                pub data: #storage_type_name,
            }

            impl #type_name {
                pub const SIZE_BYTES: usize = #byte_size;
                pub const SIZE_BITS: usize = #full_size;

                pub const MAX: Self = Self {
                    data: [#( #word_max::MAX ),*]
                };

                pub fn new(data: #storage_type_name) -> Self {
                    Self { data }
                }

                pub fn from_be_bytes(raw_data: &[u8]) -> Self {
                    if (raw_data.len() != #byte_size) {
                        panic!("should have length {}", #byte_size);
                    }
                    let mut data: #storage_type_name = Default::default();
                    #(data[#word_range_le] = #word_type_name::from_be_bytes(
                        raw_data[#word_range_le*#word_bytes..(#word_range_le + 1)*#word_bytes]
                            .try_into()
                            .expect("slice with incorrect length")
                    ));*;
                    Self::new(data)
                }

                pub fn from_le_bytes(raw_data: &[u8]) -> Self {
                    if (raw_data.len() != #byte_size) {
                        panic!("should have length {}", #byte_size);
                    }
                    let mut data: #storage_type_name = Default::default();
                    #(data[#word_range_be] = #word_type_name::from_le_bytes(
                        raw_data[#word_range_be*#word_bytes..(#word_range_be + 1)*#word_bytes]
                            .try_into()
                            .expect("slice with incorrect length")
                    ));*;
                    Self::new(data)
                }

                pub fn iter(&self) -> impl Iterator<Item = bool> + '_ {
                    (0..Self::SIZE_BITS).map(|i| self.get(i))
                }

                pub fn get(&self, idx: usize) -> bool {
                    let word = idx / #word_size;
                    let bit = (#word_size - 1) - (idx % #word_size);
                    (self.data[word] & (1 << bit) as #word_type_name) != 0
                }
            }

            pub struct #iterator_name {
                data: #type_name,
                cursor: usize,
            }

            impl #iterator_name {
                pub fn new(data: #type_name) -> Self {
                    Self { data, cursor: 0usize }
                }
            }

            impl std::iter::Iterator for #iterator_name {
                type Item = bool;

                fn next(&mut self) -> Option<Self::Item> {
                    if self.cursor == #type_name::SIZE_BITS {
                        None
                    } else {
                        let val = self.data.get(self.cursor);
                        self.cursor += 1;
                        Some(val)
                    }
                }
            }

            impl std::iter::IntoIterator for #type_name {
                type Item = bool;
                type IntoIter = #iterator_name;

                fn into_iter(self) -> Self::IntoIter {
                    #iterator_name::new(self)
                }
            }

            impl std::iter::IntoIterator for &#type_name {
                type Item = bool;
                type IntoIter = #iterator_name;

                fn into_iter(self) -> Self::IntoIter {
                    #iterator_name::new(*self)
                }
            }

            impl std::iter::FromIterator<bool> for #type_name {
                fn from_iter<T: std::iter::IntoIterator<Item = bool>>(iter: T) -> Self {
                    let mut val = Self::default();
                    for (i, el) in iter.into_iter().enumerate().take(Self::SIZE_BITS) {
                        let word = i / #word_size;
                        let bit = 1 << ((#word_size - 1) - (i % #word_size));
                        val.data[word] |= (el as #word_type_name) * bit;
                    }
                    val
                }
            }

            impl std::string::ToString for #type_name {
                fn to_string(&self) -> String {
                    let mut result = String::with_capacity(#byte_size * 2);
                    for part in self.data {
                        result.push_str(&format!("{:016X}", part))
                    }
                    result
                }
            }

            impl BitContainer for #type_name {
                type Data = #storage_type_name;

                fn data(&self) -> &Self::Data {
                    &self.data
                }

                fn data_mut(&mut self) -> &mut Self::Data {
                    &mut self.data
                }

                fn bit(&self, idx: usize) -> bool {
                    self.get(idx)
                }

                fn xor_dist(&self, other: &Self) -> u32 {
                    let mut result = 0;
                    #(result += (self.data[#word_range_xor] ^ other.data[#word_range_xor]).count_ones());*;
                    result
                }
            }
        };
        tokens.extend(code);
    }
}
