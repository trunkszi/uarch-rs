//! Source and linked-symbol checks are separate: neither proves the other.
use crate::{Error, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use syn::visit::Visit;

fn failure(detail: impl std::fmt::Display) -> Error {
    Error::Gate(format!("[audit.allocator] {detail}"))
}
fn mimalloc_path(path: &syn::Path) -> bool {
    path.segments.len() == 2
        && path
            .segments
            .iter()
            .zip(["mimalloc", "MiMalloc"])
            .all(|(s, n)| s.ident == n && matches!(s.arguments, syn::PathArguments::None))
}
fn allocator_attribute(meta: &syn::Meta) -> bool {
    if meta.path().is_ident("global_allocator") {
        return true;
    }
    if !meta.path().is_ident("cfg_attr") {
        return false;
    }
    let syn::Meta::List(list) = meta else {
        return true;
    };
    list.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
        .map_or(true, |arguments| {
            arguments.iter().skip(1).any(allocator_attribute)
        })
}
#[derive(Default)]
struct AllocatorAttributes(usize);
impl<'ast> Visit<'ast> for AllocatorAttributes {
    fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
        if allocator_attribute(&attr.meta) {
            self.0 += 1;
        }
        syn::visit::visit_attribute(self, attr);
    }
}
/// Requires exactly one unconditional, immutable, root-level mimalloc binding.
/// # Errors
/// Rejects malformed Rust, conditional files, extra attributes and nested/duplicate allocators.
pub fn verify_source(source: &str) -> Result<()> {
    let file = syn::parse_file(source).map_err(failure)?;
    if file
        .attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"))
    {
        return Err(failure("conditional file"));
    }
    let mut all = AllocatorAttributes::default();
    all.visit_file(&file);
    let valid = file.items.iter().filter(|item| {
        let syn::Item::Static(item) = item else { return false; };
        item.ident == "GLOBAL" && matches!(item.mutability, syn::StaticMutability::None)
            && item.attrs.len() == 1 && matches!(&item.attrs[0].meta, syn::Meta::Path(p) if p.is_ident("global_allocator"))
            && matches!(&*item.ty, syn::Type::Path(p) if p.qself.is_none() && mimalloc_path(&p.path))
            && matches!(&*item.expr, syn::Expr::Path(p) if p.qself.is_none() && mimalloc_path(&p.path))
    }).count();
    if valid != 1 || all.0 != 1 {
        return Err(failure(
            "expected one root GLOBAL with only #[global_allocator] and type/value mimalloc::MiMalloc",
        ));
    }
    Ok(())
}
/// Verifies defined POSIX-format nm symbols rather than undefined references.
/// # Errors
/// Rejects a missing Rust allocator, mimalloc allocation function, or mimalloc free.
pub fn verify_symbols(symbols: &str) -> Result<()> {
    let names: Vec<_> = symbols
        .lines()
        .filter_map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields.len() >= 3 && matches!(fields[1], "T" | "t" | "W" | "w") {
                Some(fields[0])
            } else {
                None
            }
        })
        .collect();
    let rust = names
        .iter()
        .any(|n| *n == "__rust_alloc" || (n.starts_with("_R") && n.ends_with("___rust_alloc")));
    if !rust
        || !names
            .iter()
            .any(|n| matches!(*n, "mi_malloc" | "mi_malloc_aligned"))
        || !names.contains(&"mi_free")
    {
        return Err(failure(
            "missing defined __rust_alloc, mi_malloc[_aligned] or mi_free",
        ));
    }
    Ok(())
}
/// Checks a linked image using the caller's LLVM tool selection.
/// # Errors
/// Rejects failed tools, malformed output or absent allocator symbols.
pub fn verify_binary(nm: &Path, binary: &Path) -> Result<()> {
    let symbols = crate::process::text(
        Command::new(nm)
            .args(["--defined-only", "--format=posix"])
            .arg(binary),
    )?;
    verify_symbols(&symbols)
}

#[derive(serde::Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}
#[derive(serde::Deserialize)]
struct Package {
    manifest_path: PathBuf,
    targets: Vec<Target>,
}
#[derive(serde::Deserialize)]
struct Target {
    kind: Vec<String>,
    src_path: PathBuf,
}
/// Locates actual Cargo binary/example targets and standalone benches in a workspace.
/// # Errors
/// Rejects metadata/tool errors, malformed manifests, missing sources or an empty target set.
pub fn executable_sources(root: &Path) -> Result<Vec<PathBuf>> {
    let mut command = Command::new("cargo");
    command
        .current_dir(root)
        .args(["metadata", "--no-deps", "--format-version=1"]);
    sources_from_metadata(&crate::process::text(&mut command)?)
}
/// Resolves metadata targets while consulting each manifest's bench harness declarations.
/// # Errors
/// Rejects invalid metadata, unreadable manifests and absent executable sources.
pub fn sources_from_metadata(json: &str) -> Result<Vec<PathBuf>> {
    let metadata: Metadata = sonic_rs::from_str(json)?;
    let mut sources = Vec::new();
    for package in metadata.packages {
        let manifest: toml::Value =
            toml::from_str(&std::fs::read_to_string(&package.manifest_path)?).map_err(failure)?;
        let parent = package
            .manifest_path
            .parent()
            .ok_or_else(|| failure("manifest has no parent"))?;
        for target in package.targets {
            if target.kind.iter().any(|k| k == "bin" || k == "example") {
                sources.push(target.src_path);
            }
        }
        if let Some(benches) = manifest.get("bench").and_then(toml::Value::as_array) {
            for bench in benches {
                if bench.get("harness").and_then(toml::Value::as_bool) != Some(false) {
                    continue;
                }
                let path = if let Some(path) = bench.get("path").and_then(toml::Value::as_str) {
                    parent.join(path)
                } else {
                    let name = bench
                        .get("name")
                        .and_then(toml::Value::as_str)
                        .ok_or_else(|| failure("unnamed standalone bench"))?;
                    let flat = parent.join("benches").join(format!("{name}.rs"));
                    if flat.is_file() {
                        flat
                    } else {
                        parent.join("benches").join(name).join("main.rs")
                    }
                };
                sources.push(path);
            }
        }
    }
    sources.sort();
    sources.dedup();
    if sources.is_empty() || sources.iter().any(|p| !p.is_file()) {
        return Err(failure("no executable sources or missing entrypoint"));
    }
    Ok(sources)
}
