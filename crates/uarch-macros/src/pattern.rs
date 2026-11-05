use std::collections::BTreeSet;

use proc_macro2::{TokenStream, TokenTree};
use quote::quote;
use syn::{
    Expr, Ident, Lit, LitStr, Path, Token, braced, parse::Parse, parse::ParseStream, parse::Parser,
    punctuated::Punctuated,
};

pub(crate) struct Input {
    root: Path,
    word: Path,
    body: TokenStream,
}

impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let root = if input.peek(Token![crate]) && input.peek2(Token![=]) {
            input.parse::<Token![crate]>()?;
            input.parse::<Token![=]>()?;
            let root = input.parse::<LitStr>()?.parse()?;
            input.parse::<Token![,]>()?;
            root
        } else {
            syn::parse_quote!(::uarch)
        };
        let word = input.call(Path::parse_mod_style)?;
        let body;
        braced!(body in input);
        let body = body.parse()?;
        Ok(Self { root, word, body })
    }
}

fn split(body: TokenStream, separator: char, trailing: bool) -> syn::Result<Vec<TokenStream>> {
    let mut parts = Vec::new();
    let mut current = TokenStream::new();
    let mut last_separator = None;
    for token in body {
        // Groups retain their nesting, so only depth-zero separators are visible.
        if let TokenTree::Punct(punct) = &token
            && punct.as_char() == separator
        {
            if current.is_empty() {
                return Err(syn::Error::new(punct.span(), "expected a bit expression"));
            }
            parts.push(current);
            current = TokenStream::new();
            last_separator = Some(punct.span());
            continue;
        }
        current.extend([token]);
        last_separator = None;
    }
    if !current.is_empty() {
        parts.push(current);
    } else if !trailing && let Some(span) = last_separator {
        return Err(syn::Error::new(span, "expected a term after &"));
    }
    if parts.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "expected at least one bit expression",
        ));
    }
    Ok(parts)
}

fn value(input: ParseStream<'_>) -> syn::Result<Expr> {
    let mut expression: Expr = input.parse()?;
    if matches!(expression, Expr::Lit(_) | Expr::Path(_) | Expr::Paren(_)) {
        // Parentheses delimit the macro grammar; method arguments supply the
        // expression boundary in the expansion without triggering unused_parens.
        while let Expr::Paren(paren) = expression {
            expression = *paren.expr;
        }
        Ok(expression)
    } else {
        Err(syn::Error::new_spanned(
            expression,
            "value must be a literal, path, or parenthesized expression",
        ))
    }
}

enum Term {
    Flag(Ident),
    Not(Ident),
    Eq(Ident, Expr),
}

impl Parse for Term {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.peek(Token![!]) {
            input.parse::<Token![!]>()?;
            return Ok(Self::Not(input.parse()?));
        }
        let name = input.parse()?;
        if input.peek(Token![==]) {
            input.parse::<Token![==]>()?;
            Ok(Self::Eq(name, value(input)?))
        } else {
            Ok(Self::Flag(name))
        }
    }
}

struct PatchItem {
    name: Ident,
    value: Expr,
}

impl Parse for PatchItem {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        Ok(Self {
            name,
            value: value(input)?,
        })
    }
}

pub(crate) fn pattern(input: Input) -> syn::Result<TokenStream> {
    let Input { root, word, body } = input;
    let mut result = quote!(#root::__private::BitPattern::<#word>::ANY);
    for part in split(body, '&', false)? {
        result = match syn::parse2::<Term>(part)? {
            Term::Flag(name) => quote!(#result.and_flag(#word::#name)),
            Term::Not(name) => quote!(#result.and_not(#word::#name)),
            Term::Eq(name, value) => quote!(#result.and_eq(#word::#name, #value)),
        };
    }
    Ok(quote!(const { #result }))
}

pub(crate) fn patch(input: Input) -> syn::Result<TokenStream> {
    let Input { root, word, body } = input;
    let mut result = quote!(#root::__private::BitPatch::<#word>::EMPTY);
    let mut seen = BTreeSet::new();
    let items = Punctuated::<PatchItem, Token![,]>::parse_terminated.parse2(body)?;
    if items.is_empty() {
        return Err(syn::Error::new_spanned(
            &word,
            "expected at least one bit expression",
        ));
    }
    for PatchItem { name, value } in items {
        if !seen.insert(name.to_string().trim_start_matches("r#").to_owned()) {
            return Err(syn::Error::new(name.span(), "duplicate patch identifier"));
        }
        result = if matches!(&value, Expr::Lit(lit) if matches!(lit.lit, Lit::Bool(_))) {
            quote!(#result.with_flag(#word::#name, #value))
        } else {
            quote!(#result.with_field(#word::#name, #value))
        };
    }
    Ok(quote!(const { #result }))
}
