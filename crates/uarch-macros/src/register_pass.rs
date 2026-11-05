use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;

use crate::{layout, paths::Options};

pub(crate) fn expand(options: Options, item: ItemStruct) -> syn::Result<TokenStream> {
    options.validate(Some("c_abi"), false)?;
    layout::validate(&item)?;
    let root = &options.root;
    let sum = layout::sum(&item);
    let proof = layout::proof(&item, root);
    let no_padding = layout::no_padding(&item, root);
    let cfg = layout::cfg_attrs(&item).collect::<Vec<_>>();
    let abi = if options.mode.is_some() {
        let types = item.fields.iter().map(|f| &f.ty);
        quote! {
            #(#root::__private::assert_copy::<#types>();)*
            ::core::assert!(::core::mem::size_of::<Self>() <= 16
                && ::core::mem::align_of::<Self>() <= 8);
            #[cfg(all(target_arch = "x86_64", any(target_os = "windows", target_os = "uefi")))]
            ::core::assert!(::core::matches!(::core::mem::size_of::<Self>(), 1 | 2 | 4 | 8));
        }
    } else {
        let types = item.fields.iter().map(|f| &f.ty).collect::<Vec<_>>();
        quote! {
            #(#root::__private::assert_register_leaf::<#types>();)*
            ::core::assert!(::core::mem::size_of::<Self>() <= 8
                || (::core::mem::size_of::<Self>() <= 16
                    && (0usize #(+ <#types as #root::__private::RegisterLeaf>::LEAVES as usize)*) == 2));
        }
    };
    let assertions = layout::assertions(
        &item,
        quote! {
            #root::__private::assert_copy::<Self>();
            #proof
            let _: [(); #sum] = [(); ::core::mem::size_of::<Self>()];
            #abi
        },
    );
    Ok(quote! {
        #[repr(C)]
        #item
        #(#cfg)*
        #assertions
        #(#cfg)*
        #no_padding
    })
}
