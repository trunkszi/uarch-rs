use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemFn;

use crate::paths::Options;

pub(crate) fn expand(options: Options, item: ItemFn, hot: bool) -> syn::Result<TokenStream> {
    options.validate(None, false)?;
    if item.sig.asyncness.is_some() || (!hot && item.sig.abi.is_some()) {
        return Err(syn::Error::new_spanned(
            &item.sig,
            "inline policies require a synchronous function; cold_sink also requires Rust ABI",
        ));
    }
    for attr in &item.attrs {
        if attr.path().is_ident("inline")
            || (hot && (attr.path().is_ident("target_feature") || attr.path().is_ident("cold")))
        {
            return Err(syn::Error::new_spanned(
                attr,
                "conflicting inline policy attribute",
            ));
        }
    }
    if hot {
        Ok(quote!(#[inline(always)] #item))
    } else {
        Ok(quote!(#[cold] #[inline(never)] #item))
    }
}
