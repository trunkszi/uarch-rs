use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;

use crate::{layout, paths::Options};

pub(crate) fn expand(options: Options, item: ItemStruct) -> syn::Result<TokenStream> {
    options.validate(None, true)?;
    layout::validate(&item)?;
    let lines = match &options.lines {
        Some(value) => value.base10_parse::<usize>()?,
        None => 1,
    };
    if lines == 0 || 64usize.checked_mul(lines).is_none() {
        return Err(syn::Error::new_spanned(
            options.lines,
            "lines must be positive and 64 * lines must fit usize",
        ));
    }
    let root = &options.root;
    let sum = layout::sum(&item);
    let proof = layout::proof(&item, root);
    let no_padding = layout::no_padding(&item, root);
    let cfg = layout::cfg_attrs(&item).collect::<Vec<_>>();
    let assertions = layout::assertions(
        &item,
        quote! {
            ::core::assert!(64usize.checked_mul(#lines).is_some());
            #proof
            let _: [(); 64usize * #lines] = [(); #sum];
            const { ::core::assert!(::core::mem::size_of::<Self>() == 64usize * #lines
                && ::core::mem::align_of::<Self>() == 64); }
        },
    );
    Ok(quote! {
        #[repr(C, align(64))]
        #item
        #(#cfg)*
        #assertions
        #(#cfg)*
        #no_padding
    })
}
