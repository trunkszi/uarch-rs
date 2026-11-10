//! Layout proofs, inline policies, target-tier dispatch, and const bit expressions.
extern crate proc_macro;

mod cache_line;
mod inline_policy;
mod isolated;
mod layout;
mod multiversion;
mod paths;
mod pattern;
mod register_pass;

use proc_macro::TokenStream;

fn output(result: syn::Result<proc_macro2::TokenStream>) -> TokenStream {
    result.unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Prove a named-field aggregate is padding-free and eligible for a small ABI.
#[proc_macro_attribute]
pub fn register_pass(attr: TokenStream, item: TokenStream) -> TokenStream {
    output(syn::parse(attr).and_then(|options| register_pass::expand(options, syn::parse(item)?)))
}

/// Prove a named-field aggregate fills an exact number of 64-byte lines.
#[proc_macro_attribute]
pub fn cache_line(attr: TokenStream, item: TokenStream) -> TokenStream {
    output(syn::parse(attr).and_then(|options| cache_line::expand(options, syn::parse(item)?)))
}

/// Isolate an aggregate at 128-byte alignment, optionally proving full occupancy.
#[proc_macro_attribute]
pub fn isolated(attr: TokenStream, item: TokenStream) -> TokenStream {
    output(syn::parse(attr).and_then(|options| isolated::expand(options, syn::parse(item)?)))
}

/// Always inline a function without an explicit target-feature boundary.
#[proc_macro_attribute]
pub fn hot_inline(attr: TokenStream, item: TokenStream) -> TokenStream {
    output(
        syn::parse(attr)
            .and_then(|options| inline_policy::expand(options, syn::parse(item)?, true)),
    )
}

/// Keep a cold function outside the hot inline path.
#[proc_macro_attribute]
pub fn cold_sink(attr: TokenStream, item: TokenStream) -> TokenStream {
    output(
        syn::parse(attr)
            .and_then(|options| inline_policy::expand(options, syn::parse(item)?, false)),
    )
}

/// Specialize an Arch-generic free function for all supported hardware tiers.
#[proc_macro_attribute]
pub fn multiversion(attr: TokenStream, item: TokenStream) -> TokenStream {
    output(syn::parse(attr).and_then(|options| multiversion::expand(options, syn::parse(item)?)))
}

/// Build a const bit pattern from flag and equality terms separated by `&`.
#[proc_macro]
pub fn pattern(input: TokenStream) -> TokenStream {
    output(syn::parse(input).and_then(pattern::pattern))
}

/// Build a const bit patch from unique flag or field assignments.
#[proc_macro]
pub fn patch(input: TokenStream) -> TokenStream {
    output(syn::parse(input).and_then(pattern::patch))
}
