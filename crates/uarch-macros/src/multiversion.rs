use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, FnArg, GenericParam, Ident, ItemFn, Pat, Path, ReturnType, Token, Type,
    TypeParamBound, WherePredicate, parse_quote,
    visit::{self, Visit},
};

use crate::paths::Options;

struct TypeCheck<'a> {
    arch: &'a Ident,
    indirect: usize,
    error: Option<syn::Error>,
}

impl<'ast> Visit<'ast> for TypeCheck<'_> {
    fn visit_path(&mut self, path: &'ast Path) {
        if path.leading_colon.is_none()
            && path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == *self.arch)
        {
            self.error = Some(syn::Error::new_spanned(
                path,
                "arch type cannot occur in other parameters or the return type",
            ));
        }
        visit::visit_path(self, path);
    }

    fn visit_type(&mut self, ty: &'ast Type) {
        if matches!(ty, Type::ImplTrait(_)) {
            self.error = Some(syn::Error::new_spanned(
                ty,
                "impl Trait is outside the multiversion signature subset",
            ));
        }
        if self.indirect == 0
            && let Type::Path(path) = ty
        {
            for segment in &path.path.segments {
                let name = segment.ident.to_string();
                if name.starts_with("__m128")
                    || name.starts_with("__m256")
                    || name.starts_with("__m512")
                    || name == "Simd"
                    || (name.ends_with("_t")
                        && name.contains('x')
                        && ["int", "uint", "float", "poly", "bfloat"]
                            .iter()
                            .any(|prefix| name.starts_with(prefix)))
                {
                    self.error = Some(syn::Error::new_spanned(
                        ty,
                        "by-value SIMD is outside the multiversion signature subset",
                    ));
                }
            }
        }
        let indirect = matches!(ty, Type::Reference(_) | Type::Ptr(_));
        self.indirect += usize::from(indirect);
        visit::visit_type(self, ty);
        self.indirect -= usize::from(indirect);
    }
}

fn forbidden_attribute(attr: &Attribute) -> bool {
    if [
        "target_feature",
        "cfg_attr",
        "hot_inline",
        "inline",
        "no_mangle",
        "export_name",
    ]
    .iter()
    .any(|name| attr.path().is_ident(name))
    {
        return true;
    }
    if attr.path().is_ident("unsafe") {
        let tokens = quote!(#attr).to_string();
        return tokens.contains("no_mangle") || tokens.contains("export_name");
    }
    false
}

fn validate(item: &ItemFn) -> syn::Result<Ident> {
    let signature = &item.sig;
    if signature.asyncness.is_some()
        || signature.constness.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
    {
        return Err(syn::Error::new_spanned(
            signature,
            "multiversion requires an ordinary non-const, non-async Rust function",
        ));
    }
    for attr in &item.attrs {
        if forbidden_attribute(attr) {
            return Err(syn::Error::new_spanned(
                attr,
                "attribute conflicts with generated multiversion clones",
            ));
        }
    }
    let mut arch = None;
    for parameter in &signature.generics.params {
        match parameter {
            GenericParam::Lifetime(_) => {}
            GenericParam::Type(parameter) if arch.is_none() && parameter.default.is_none() => {
                if parameter.bounds.len() != 1 || !parameter.bounds.iter().any(|bound| {
                    matches!(bound, TypeParamBound::Trait(bound)
                        if matches!(bound.modifier, syn::TraitBoundModifier::None)
                        && bound.lifetimes.is_none()
                        && bound.path.segments.last().is_some_and(|segment| segment.ident == "Arch" && segment.arguments.is_empty()))
                }) {
                    return Err(syn::Error::new_spanned(parameter, "expected a single type parameter A: Arch"));
                }
                arch = Some(parameter.ident.clone());
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    parameter,
                    "only one Arch type parameter and explicit lifetimes are supported",
                ));
            }
        }
    }
    let arch = arch.ok_or_else(|| {
        syn::Error::new_spanned(signature, "expected a single type parameter A: Arch")
    })?;
    if let Some(clause) = &signature.generics.where_clause {
        for predicate in &clause.predicates {
            if !matches!(predicate, WherePredicate::Lifetime(_)) {
                return Err(syn::Error::new_spanned(
                    predicate,
                    "only lifetime where-predicates are supported",
                ));
            }
        }
    }
    let mut inputs = signature.inputs.iter();
    let Some(FnArg::Typed(first)) = inputs.next() else {
        return Err(syn::Error::new_spanned(
            signature,
            "first parameter must be arch: A on a free function",
        ));
    };
    if !matches!(&*first.pat, Pat::Ident(pat) if pat.ident == "arch" && pat.by_ref.is_none() && pat.subpat.is_none())
        || !matches!(&*first.ty, Type::Path(ty) if ty.qself.is_none() && ty.path.is_ident(&arch))
    {
        return Err(syn::Error::new_spanned(
            first,
            "first parameter must be arch: A",
        ));
    }
    let mut check = TypeCheck {
        arch: &arch,
        indirect: 0,
        error: None,
    };
    for input in inputs {
        let FnArg::Typed(input) = input else {
            return Err(syn::Error::new_spanned(
                input,
                "multiversion supports free functions only",
            ));
        };
        if !input.attrs.is_empty() {
            return Err(syn::Error::new_spanned(
                input,
                "parameter attributes are outside the multiversion signature subset",
            ));
        }
        check.visit_type(&input.ty);
    }
    if let ReturnType::Type(_, ty) = &signature.output {
        check.visit_type(ty);
    }
    if let Some(error) = check.error {
        return Err(error);
    }
    Ok(arch)
}

pub(crate) fn expand(options: Options, mut item: ItemFn) -> syn::Result<TokenStream> {
    options.validate(None, false)?;
    let arch = validate(&item)?;
    let root = &options.root;
    let original = item.clone();
    let name = &original.sig.ident;
    let cfg = original
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("cfg"))
        .collect::<Vec<_>>();
    let mut args = Vec::new();
    for (index, input) in item.sig.inputs.iter_mut().enumerate() {
        if let FnArg::Typed(input) = input {
            let ident = if index == 0 {
                format_ident!("arch")
            } else {
                format_ident!("__uarch_arg_{index}")
            };
            *input.pat = parse_quote!(#ident);
            args.push(ident);
        }
    }
    let forwarded = args.iter().skip(1).collect::<Vec<_>>();
    let tiers = [
        ("Scalar", "scalar", None, None),
        (
            "X86V3",
            "x86_v3",
            Some("x86_64"),
            Some(uarch_tiers::X86_V3_FEATURES),
        ),
        (
            "X86V3SlowPdep",
            "x86_v3_slow_pdep",
            Some("x86_64"),
            Some(uarch_tiers::X86_V3_FEATURES),
        ),
        (
            "X86V4",
            "x86_v4",
            Some("x86_64"),
            Some(uarch_tiers::X86_V4_FEATURES),
        ),
        (
            "Aarch64Neon",
            "aarch64_neon",
            Some("aarch64"),
            Some(uarch_tiers::AARCH64_NEON_FEATURES),
        ),
    ];
    let mut clones = TokenStream::new();
    let mut arms = TokenStream::new();
    for (variant, suffix, target, features) in tiers {
        let variant = Ident::new(variant, name.span());
        let clone_name = format_ident!("__uarch_{name}_{suffix}");
        let concrete: Path = parse_quote!(#root::__private::#variant);
        let mut clone = original.clone();
        clone.vis = syn::Visibility::Inherited;
        clone.attrs = original
            .attrs
            .iter()
            .filter(|a| a.path().is_ident("cfg") || a.path().is_ident("allow"))
            .cloned()
            .collect();
        clone.sig.ident = clone_name.clone();
        clone.sig.unsafety = Some(Token![unsafe](name.span()));
        clone.sig.generics.params = clone
            .sig
            .generics
            .params
            .into_iter()
            .filter(|p| matches!(p, GenericParam::Lifetime(_)))
            .collect();
        if clone.sig.generics.params.is_empty() {
            clone.sig.generics.lt_token = None;
            clone.sig.generics.gt_token = None;
        }
        if let Some(FnArg::Typed(first)) = clone.sig.inputs.first_mut() {
            *first.ty = parse_quote!(#concrete);
        }
        // A local alias also resolves references inside opaque macro bodies,
        // while respecting ordinary lexical type shadowing within the function.
        clone
            .block
            .stmts
            .insert(0, parse_quote!(#[allow(dead_code)] type #arch = #concrete;));
        let target_cfg = target.map(|target| quote!(#[cfg(target_arch = #target)]));
        let feature = features.map(|features| quote!(#[target_feature(enable = #features)]));
        // SAFETY: The sealed Arch::TIER match establishes the concrete token's
        // capability contract, including the CPU execution domain, before calls.
        clones.extend(quote! {
            #target_cfg
            #feature
            #[inline]
            #[allow(clippy::undocumented_unsafe_blocks, reason = "generated token dispatch preserves the sealed Arch capability proof")]
            #clone
        });
        arms.extend(quote! {
            #target_cfg
            #root::__private::TierId::#variant => {
                unsafe {
                    #clone_name(#root::__private::retag_unchecked::<#arch, #concrete>(arch), #(#forwarded),*)
                }
            },
        });
        if let Some(target) = target {
            arms.extend(quote! {
                #[cfg(not(target_arch = #target))]
                #root::__private::TierId::#variant => ::core::unreachable!("tier unavailable on this target"),
            });
        }
    }
    *item.block = parse_quote!({
        match <#arch as #root::__private::Arch>::TIER { #arms }
    });
    Ok(quote! {
        #(#cfg)*
        #[inline(always)]
        #[allow(clippy::undocumented_unsafe_blocks, reason = "sealed Arch::TIER proves that retag and target-feature calls are supported")]
        #item
        #clones
    })
}
