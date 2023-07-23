use quote::{quote, ToTokens};
use syn::Ident;

pub struct BitOp<'a> {
    pub op: bit_permute::BitOp,
    word_type_name: &'a Ident,
}

impl<'a> BitOp<'a> {
    pub fn new(op: bit_permute::BitOp, word_type_name: &'a Ident) -> Self {
        Self { op, word_type_name }
    }
}

impl ToTokens for BitOp<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let word_type_name = self.word_type_name;
        let op_tokens = match self.op {
            bit_permute::BitOp::MaskShiftAndCopy {
                src_word,
                src_mask,
                src_shift,
                dst_word,
            } => {
                let shift = src_shift.abs();
                if src_shift < 0 {
                    quote! {
                        nw.data[#dst_word] |= (w.data[#src_word] & (#src_mask as #word_type_name)) >> #shift
                    }
                } else {
                    quote! {
                        nw.data[#dst_word] |= (w.data[#src_word] & (#src_mask as #word_type_name)) << #shift
                    }
                }
            }
            bit_permute::BitOp::MaskAndCopy {
                src_word,
                src_mask,
                dst_word,
            } => quote! {
                nw.data[#dst_word] |= w.data[#src_word] & (#src_mask as #word_type_name)
            },
            bit_permute::BitOp::Copy { src_word, dst_word } => quote! {
                nw.data[#dst_word] = w.data[#src_word]
            },
        };
        tokens.extend(op_tokens);
    }
}
