mod bit_op;
mod bits;
mod permutation;

extern crate proc_macro;

use bit_permute::create_permutations;
use darling::FromMeta;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, AttributeArgs, Ident};

use crate::{bits::Bits, permutation::Permutation};

#[derive(FromMeta)]
struct PermutationParams {
    struct_name: Ident,
    f: usize,
    r: usize,
    k: usize,
    w: Option<usize>,
}

#[proc_macro]
pub fn make_permutations(item: TokenStream) -> TokenStream {
    let attr_args = parse_macro_input!(item as AttributeArgs);
    let params = match PermutationParams::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    let word_bits = params.w.unwrap_or(64);
    assert!(
        [8, 16, 32, 64].contains(&word_bits),
        "word size {} is not supported",
        word_bits
    );
    let n_words = params.f / word_bits;
    assert!(params.f % word_bits == 0 && n_words > 0);

    let struct_name = params.struct_name;
    let util_struct_name = format_ident!("{}Util", struct_name);
    let data_type_name = format_ident!("Bits");
    let mask_type_name = format_ident!("Mask");
    let word_type_name = format_ident!("u{}", word_bits);

    let perms = create_permutations(params.f, word_bits, params.r, params.k);

    let bits_definition = Bits::new(&data_type_name, &word_type_name, word_bits, n_words);

    let mask_size = perms
        .iter()
        .map(|p| p.mask_words(word_bits))
        .max()
        .unwrap_or(0);
    let mask_definition = Bits::new(&mask_type_name, &word_type_name, word_bits, mask_size);

    let perms_definitions = perms
        .into_iter()
        .enumerate()
        .map(|(i, perm)| {
            Permutation::new(
                perm,
                format_ident!("{}{}", struct_name, i),
                &data_type_name,
                &mask_type_name,
                &word_type_name,
                word_bits,
            )
        })
        .collect::<Vec<_>>();

    let variants_range = 0..perms_definitions.len();
    let variants = perms_definitions.iter().map(|p| p.struct_name.clone());
    let all_variants_range = variants_range.clone();

    quote! {
        pub use __permutations::{#data_type_name, #mask_type_name, #struct_name, #util_struct_name};

        mod __permutations {
            #bits_definition

            #mask_definition

            pub trait #struct_name: Send + Sync {
                fn apply(&self, w: #data_type_name) -> #data_type_name;

                fn revert(&self, w: #data_type_name) -> #data_type_name;

                fn mask(&self, w: &#data_type_name) -> #mask_type_name;
            }

            pub struct #util_struct_name;

            impl #util_struct_name {
                #[inline(always)]
                pub fn get_variant(variant: usize) -> std::sync::Arc<dyn #struct_name> {
                    match variant {
                        #( #variants_range => std::sync::Arc::new(#variants {}) as std::sync::Arc<dyn #struct_name> ),*,
                        i => panic!("permutation variant out of range: {}", i),
                    }
                }

                #[inline(always)]
                pub fn get_all_variants() -> Vec<std::sync::Arc<dyn #struct_name>> {
                    vec![
                        #( Self::get_variant(#all_variants_range) ),*
                    ]
                }
            }

            #(#perms_definitions)*
        }
    }
    .into()
}
