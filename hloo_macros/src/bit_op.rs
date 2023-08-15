use darling::export::syn::Ident;
use quote::{quote, ToTokens};

pub struct BitOp<'a> {
    pub op: hloo_core::BitOp,
    word_type_name: &'a Ident,
}

impl<'a> BitOp<'a> {
    pub fn new(op: hloo_core::BitOp, word_type_name: &'a Ident) -> Self {
        Self { op, word_type_name }
    }
}

impl ToTokens for BitOp<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let word_type_name = self.word_type_name;
        let op_tokens = match self.op {
            hloo_core::BitOp::MaskShiftAndCopy {
                src_word,
                src_mask,
                src_shift,
                dst_word,
            } => {
                let shift = src_shift.abs();
                if src_shift < 0 {
                    quote! {
                        out[#dst_word] |= (inp[#src_word] & (#src_mask as #word_type_name)) >> #shift
                    }
                } else {
                    quote! {
                        out[#dst_word] |= (inp[#src_word] & (#src_mask as #word_type_name)) << #shift
                    }
                }
            }
            hloo_core::BitOp::MaskAndCopy {
                src_word,
                src_mask,
                dst_word,
            } => quote! {
                out[#dst_word] |= inp[#src_word] & (#src_mask as #word_type_name)
            },
            hloo_core::BitOp::Copy { src_word, dst_word } => quote! {
                out[#dst_word] = inp[#src_word]
            },
        };
        tokens.extend(op_tokens);
    }
}
