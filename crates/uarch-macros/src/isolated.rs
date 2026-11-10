use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;

use crate::{layout, paths::Options};

pub(crate) fn expand(options: Options, item: ItemStruct) -> syn::Result<TokenStream> {
    options.validate(Some("full"), false)?;
    layout::validate(&item)?;
    let root = &options.root;
    let cfg = layout::cfg_attrs(&item).collect::<Vec<_>>();
    let (proof, implementation) = if options.mode.is_some() {
        let sum = layout::sum(&item);
        let proof = layout::proof(&item, root);
        let implementation = layout::no_padding(&item, root);
        (
            quote! {
                #proof
                let _: [(); 128] = [(); #sum];
                ::core::assert!(::core::mem::size_of::<Self>() == 128);
            },
            quote!(#(#cfg)* #implementation),
        )
    } else {
        (TokenStream::new(), TokenStream::new())
    };
    let assertions = layout::assertions(
        &item,
        quote! {
            const { ::core::assert!(::core::mem::align_of::<Self>() == 128); }
            #proof
        },
    );
    Ok(quote! {
        #[repr(C, align(128))]
        #item
        #(#cfg)*
        #assertions
        #implementation
    })
}
