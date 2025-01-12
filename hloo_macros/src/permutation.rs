use darling::{export::syn::Ident, ToTokens};
use quote::quote;

use crate::bit_op::BitOp;

pub struct Permutation<'a> {
    pub perm: hloo_core::Permutation,
    pub struct_name: Ident,
    data_type_name: &'a Ident,
    mask_type_name: &'a Ident,
    word_type_name: &'a Ident,
    word_size: usize,
}

impl<'a> Permutation<'a> {
    pub fn new(
        perm: hloo_core::Permutation,
        struct_name: Ident,
        data_type_name: &'a Ident,
        mask_type_name: &'a Ident,
        word_type_name: &'a Ident,
        word_size: usize,
    ) -> Self {
        Self {
            perm,
            struct_name,
            data_type_name,
            mask_type_name,
            word_type_name,
            word_size,
        }
    }
}

impl ToTokens for Permutation<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let apply_ops = self
            .perm
            .compile_apply(self.word_size, true)
            .into_iter()
            .flat_map(|(_, ops)| ops)
            .map(|op| BitOp::new(op, self.word_type_name))
            .collect::<Vec<_>>();

        let revert_ops = self
            .perm
            .compile_revert(self.word_size, true)
            .into_iter()
            .flat_map(|(_, ops)| ops)
            .map(|op| BitOp::new(op, self.word_type_name))
            .collect::<Vec<_>>();

        let mask_ops = self
            .perm
            .compile_top_mask(self.word_size, true)
            .into_iter()
            .flat_map(|(_, ops)| ops)
            .map(|op| BitOp::new(op, self.word_type_name))
            .collect::<Vec<_>>();

        let struct_name = &self.struct_name;
        let data_type_name = self.data_type_name;
        let mask_type_name = self.mask_type_name;
        let n_blocks = self.perm.blocks().len();

        let code = quote! {
            #[derive(Clone, Copy)]
            pub struct #struct_name;

            impl BitPermuter<#data_type_name, #mask_type_name> for #struct_name {
                fn apply_static(w: &#data_type_name) -> #data_type_name {
                    let mut nw: #data_type_name = Default::default();
                    let (inp, mut out) = (w.data(), nw.data_mut());
                    #(#apply_ops);*;
                    nw
                }

                fn revert_static(w: &#data_type_name) -> #data_type_name {
                    let mut nw: #data_type_name = Default::default();
                    let (inp, mut out) = (w.data(), nw.data_mut());
                    #(#revert_ops);*;
                    nw
                }

                fn mask_static(w: &#data_type_name) -> #mask_type_name {
                    let mut nw: #mask_type_name = Default::default();
                    let (inp, mut out) = (w.data(), nw.data_mut());
                    #(#mask_ops);*;
                    nw
                }

                fn apply(&self, w: &#data_type_name) -> #data_type_name {
                    Self::apply_static(w)
                }

                fn revert(&self, w: &#data_type_name) -> #data_type_name {
                    Self::revert_static(w)
                }

                fn mask(&self, w: &#data_type_name) -> #mask_type_name {
                    Self::mask_static(w)
                }

                fn mask_and_cmp(&self, w: &#data_type_name, other_mask: &#mask_type_name) -> std::cmp::Ordering {
                    Self::mask_static(w).cmp(other_mask)
                }

                fn n_blocks(&self) -> u32 {
                    #n_blocks as u32
                }
            }
        };
        tokens.extend(code);
    }
}
