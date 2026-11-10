use std::collections::BTreeSet;

use syn::{Ident, LitInt, LitStr, Path, Token, ext::IdentExt, parse::Parse, parse::ParseStream};

pub(crate) struct Options {
    pub(crate) root: Path,
    pub(crate) mode: Option<Ident>,
    pub(crate) lines: Option<LitInt>,
}

impl Parse for Options {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut result = Self {
            root: syn::parse_quote!(::uarch),
            mode: None,
            lines: None,
        };
        let mut seen = BTreeSet::new();
        while !input.is_empty() {
            let key = Ident::parse_any(input)?;
            if !seen.insert(key.to_string()) {
                return Err(syn::Error::new(key.span(), "duplicate macro option"));
            }
            match key.to_string().as_str() {
                "crate" => {
                    input.parse::<Token![=]>()?;
                    result.root = input.parse::<LitStr>()?.parse()?;
                    if result.root.segments.iter().any(|s| !s.arguments.is_empty()) {
                        return Err(syn::Error::new(
                            key.span(),
                            "crate path cannot have arguments",
                        ));
                    }
                }
                "lines" => {
                    input.parse::<Token![=]>()?;
                    result.lines = Some(input.parse()?);
                }
                "c_abi" | "full" => {
                    if result.mode.is_some() {
                        return Err(syn::Error::new(key.span(), "conflicting macro modes"));
                    }
                    result.mode = Some(key);
                }
                _ => return Err(syn::Error::new(key.span(), "unknown macro option")),
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(result)
    }
}

impl Options {
    pub(crate) fn validate(&self, mode: Option<&str>, lines: bool) -> syn::Result<()> {
        if let Some(actual) = &self.mode
            && mode.is_none_or(|allowed| actual != allowed)
        {
            return Err(syn::Error::new(actual.span(), "unsupported macro mode"));
        }
        if let Some(value) = &self.lines
            && !lines
        {
            return Err(syn::Error::new(
                value.span(),
                "lines is only supported by cache_line",
            ));
        }
        Ok(())
    }
}
