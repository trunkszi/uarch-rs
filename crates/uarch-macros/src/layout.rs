use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, ItemStruct, Path};

pub(crate) fn validate(item: &ItemStruct) -> syn::Result<()> {
    if !item.generics.params.is_empty() || item.generics.where_clause.is_some() {
        return Err(syn::Error::new_spanned(
            &item.generics,
            "layout macros reject generics and lifetimes",
        ));
    }
    if !matches!(item.fields, Fields::Named(_)) {
        return Err(syn::Error::new_spanned(
            &item.fields,
            "layout macros require a named-field struct",
        ));
    }
    for attr in &item.attrs {
        if attr.path().is_ident("repr") {
            return Err(syn::Error::new_spanned(
                attr,
                "layout macros supply repr; remove the user repr",
            ));
        }
        if attr.path().is_ident("cfg_attr") {
            return Err(syn::Error::new_spanned(
                attr,
                "conditional struct attributes cannot change layout",
            ));
        }
    }
    for field in &item.fields {
        for attr in &field.attrs {
            if attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr") {
                return Err(syn::Error::new_spanned(
                    attr,
                    "conditional fields cannot be layout-proven",
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn sum(item: &ItemStruct) -> TokenStream {
    let types = item.fields.iter().map(|f| &f.ty);
    quote!(0usize #(+ ::core::mem::size_of::<#types>())*)
}

pub(crate) fn proof(item: &ItemStruct, root: &Path) -> TokenStream {
    let mut offset = quote!(0usize);
    let mut checks = TokenStream::new();
    for field in &item.fields {
        let ty = &field.ty;
        let member = &field.ident;
        checks.extend(quote! {
            #root::__private::assert_no_padding::<#ty>();
            ::core::assert!(::core::mem::offset_of!(Self, #member) == #offset);
        });
        offset = quote!(#offset + ::core::mem::size_of::<#ty>());
    }
    checks
}

pub(crate) fn no_padding(item: &ItemStruct, root: &Path) -> TokenStream {
    let name = &item.ident;
    // SAFETY: Every field is padding-free, field offsets are contiguous, and the
    // const size equality proves the absence of trailing padding.
    quote! {
        #[automatically_derived]
        unsafe impl #root::__private::NoPadding for #name {}
    }
}

pub(crate) fn cfg_attrs(item: &ItemStruct) -> impl Iterator<Item = &syn::Attribute> {
    item.attrs.iter().filter(|attr| attr.path().is_ident("cfg"))
}

pub(crate) fn assertions(item: &ItemStruct, checks: TokenStream) -> TokenStream {
    let name = &item.ident;
    // The local impl gives field types containing Self their original type scope
    // without exposing an associated proof item on the user's struct.
    quote! {
        const _: () = {
            trait __UarchLayoutProof { const CHECK: (); }
            impl __UarchLayoutProof for #name {
                const CHECK: () = { #checks };
            }
            let () = <#name as __UarchLayoutProof>::CHECK;
        };
    }
}
