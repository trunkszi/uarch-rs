use crate::{Error, Result, process};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use syn::visit::{self, Visit};

fn files_under(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            files_under(&path, files)?;
        } else if entry.file_type()?.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

#[derive(Default)]
struct Violations {
    descriptor: bool,
    chinese: bool,
    asynchronous: bool,
    domain: bool,
    runtime_pattern: bool,
    extraction: bool,
    target_feature: bool,
    test_name: bool,
}

struct RustAudit {
    violations: Violations,
    production: bool,
    ui: bool,
    descriptors: bool,
    patterns: bool,
    audited_arch: bool,
    tests: bool,
    const_depth: usize,
}

fn name(identifier: &syn::Ident) -> String {
    identifier.to_string().trim_start_matches("r#").to_owned()
}

fn is_han(ch: char) -> bool {
    matches!(ch as u32,
        0x3000..=0x303f | 0x3400..=0x4dbf | 0x4e00..=0x9fff
        | 0xf900..=0xfaff | 0xff00..=0xffef | 0x20000..=0x323af)
}

fn han_in_literals(tokens: proc_macro2::TokenStream) -> bool {
    tokens.into_iter().any(|token| match token {
        proc_macro2::TokenTree::Group(group) => han_in_literals(group.stream()),
        proc_macro2::TokenTree::Literal(literal) => {
            match syn::parse_str::<syn::Lit>(&literal.to_string()) {
                Ok(syn::Lit::Str(value)) => value.value().chars().any(is_han),
                Ok(syn::Lit::Char(value)) => is_han(value.value()),
                _ => false,
            }
        }
        _ => false,
    })
}

fn has_identifier(tokens: proc_macro2::TokenStream, expected: &str) -> bool {
    tokens.into_iter().any(|token| match token {
        proc_macro2::TokenTree::Ident(identifier) => name(&identifier) == expected,
        proc_macro2::TokenTree::Group(group) => has_identifier(group.stream(), expected),
        _ => false,
    })
}

fn punctuation(token: Option<&proc_macro2::TokenTree>, expected: char) -> bool {
    matches!(token, Some(proc_macro2::TokenTree::Punct(punct)) if punct.as_char() == expected)
}

fn descriptor_tokens(tokens: &[proc_macro2::TokenTree]) -> bool {
    if !punctuation(tokens.get(1), ':') || !punctuation(tokens.get(2), ':') {
        return false;
    }
    let mut index = 3;
    if punctuation(tokens.get(index), '<') {
        let mut depth = 0;
        while let Some(token) = tokens.get(index) {
            if punctuation(Some(token), '<') {
                depth += 1;
            } else if punctuation(Some(token), '>')
                && !punctuation(index.checked_sub(1).and_then(|i| tokens.get(i)), '-')
            {
                depth -= 1;
                if depth == 0 {
                    index += 1;
                    break;
                }
            }
            index += 1;
        }
        if !punctuation(tokens.get(index), ':') || !punctuation(tokens.get(index + 1), ':') {
            return false;
        }
        index += 2;
    }
    matches!(tokens.get(index), Some(proc_macro2::TokenTree::Ident(identifier)) if identifier == "new")
}

fn descriptor_call(expression: &syn::Expr) -> bool {
    match expression {
        syn::Expr::Paren(expression) => descriptor_call(&expression.expr),
        syn::Expr::Group(expression) => descriptor_call(&expression.expr),
        syn::Expr::Path(expression) => {
            let mut segments = expression.path.segments.iter().rev();
            if !segments
                .next()
                .is_some_and(|segment| segment.ident == "new")
            {
                return false;
            }
            if let Some(owner) = segments.next() {
                return matches!(name(&owner.ident).as_str(), "BitFlag" | "BitField");
            }
            expression.qself.as_ref().is_some_and(|qualified| {
                matches!(qualified.ty.as_ref(), syn::Type::Path(owner)
                    if owner.path.segments.last().is_some_and(|segment|
                        matches!(name(&segment.ident).as_str(), "BitFlag" | "BitField")))
            })
        }
        _ => false,
    }
}

impl RustAudit {
    // Macro inputs are opaque to syn's expression visitor. Lexical checks still
    // inspect their tokens without interpreting quoted strings as Rust code.
    fn tokens(&mut self, tokens: proc_macro2::TokenStream) {
        let tokens: Vec<_> = tokens.into_iter().collect();
        for (index, token) in tokens.iter().enumerate() {
            match token {
                proc_macro2::TokenTree::Group(group) => self.tokens(group.stream()),
                proc_macro2::TokenTree::Ident(identifier) => {
                    self.visit_ident(identifier);
                    let identifier = name(identifier);
                    if !self.ui && matches!(identifier.as_str(), "async" | "await") {
                        self.violations.asynchronous = true;
                    }
                    if self.production
                        && matches!(identifier.as_str(), "unwrap" | "expect")
                        && (punctuation(index.checked_sub(1).and_then(|i| tokens.get(i)), '.')
                            || (punctuation(index.checked_sub(1).and_then(|i| tokens.get(i)), ':')
                                && punctuation(
                                    index.checked_sub(2).and_then(|i| tokens.get(i)),
                                    ':',
                                )))
                        && matches!(tokens.get(index + 1), Some(proc_macro2::TokenTree::Group(group))
                            if group.delimiter() == proc_macro2::Delimiter::Parenthesis)
                    {
                        self.violations.extraction = true;
                    }
                    if self.descriptors
                        && matches!(identifier.as_str(), "BitFlag" | "BitField")
                        && descriptor_tokens(&tokens[index..])
                    {
                        self.violations.descriptor = true;
                    }
                }
                proc_macro2::TokenTree::Literal(literal) => {
                    match syn::parse_str::<syn::Lit>(&literal.to_string()) {
                        Ok(syn::Lit::Str(literal)) => self.visit_lit_str(&literal),
                        Ok(syn::Lit::Char(literal)) => self.visit_lit_char(&literal),
                        _ => {}
                    }
                }
                proc_macro2::TokenTree::Punct(_) => {}
            }
        }
    }
}

impl<'ast> Visit<'ast> for RustAudit {
    fn visit_ident(&mut self, identifier: &'ast syn::Ident) {
        if matches!(name(identifier).as_str(), "venue" | "model_zoo") {
            self.violations.domain = true;
        }
    }

    fn visit_lit_str(&mut self, literal: &'ast syn::LitStr) {
        if literal.value().chars().any(is_han) {
            self.violations.chinese = true;
        }
    }

    fn visit_lit_char(&mut self, literal: &'ast syn::LitChar) {
        if is_han(literal.value()) {
            self.violations.chinese = true;
        }
    }

    fn visit_signature(&mut self, signature: &'ast syn::Signature) {
        if !self.ui && signature.asyncness.is_some() {
            self.violations.asynchronous = true;
        }
        visit::visit_signature(self, signature);
    }

    fn visit_expr(&mut self, expression: &'ast syn::Expr) {
        if !self.ui
            && (matches!(expression, syn::Expr::Async(_) | syn::Expr::Await(_))
                || matches!(expression, syn::Expr::Closure(closure) if closure.asyncness.is_some()))
        {
            self.violations.asynchronous = true;
        }
        visit::visit_expr(self, expression);
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if self.descriptors && descriptor_call(&call.func) {
            self.violations.descriptor = true;
        }
        if self.production
            && matches!(call.func.as_ref(), syn::Expr::Path(path)
                if path.path.segments.last().is_some_and(|segment|
                    matches!(name(&segment.ident).as_str(), "unwrap" | "expect")))
        {
            self.violations.extraction = true;
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        let method = name(&call.method);
        if self.production && (method == "expect" || (method == "unwrap" && call.args.is_empty())) {
            self.violations.extraction = true;
        }
        if self.patterns
            && self.const_depth == 0
            && matches!(
                method.as_str(),
                "and_flag" | "and_not" | "and_eq" | "with_flag" | "with_field"
            )
        {
            self.violations.runtime_pattern = true;
        }
        visit::visit_expr_method_call(self, call);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.tokens(mac.tokens.clone());
        visit::visit_macro(self, mac);
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        // The default item traversal delegates its body to visit_macro.
        visit::visit_item_macro(self, item);
    }

    fn visit_attribute(&mut self, attribute: &'ast syn::Attribute) {
        if let syn::Meta::List(list) = &attribute.meta
            && han_in_literals(list.tokens.clone())
        {
            self.violations.chinese = true;
        }
        if self.production
            && !self.audited_arch
            && (attribute.path().is_ident("target_feature")
                || (attribute.path().is_ident("cfg_attr")
                    && matches!(&attribute.meta, syn::Meta::List(meta)
                        if has_identifier(meta.tokens.clone(), "target_feature"))))
        {
            self.violations.target_feature = true;
        }
        visit::visit_attribute(self, attribute);
    }

    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        let identifier = name(&function.sig.ident);
        if self.tests && !self.ui && (identifier.len() > 48 || identifier.split('_').count() > 6) {
            self.violations.test_name = true;
        }
        let saved = self.const_depth;
        self.const_depth = 0;
        visit::visit_item_fn(self, function);
        self.const_depth = saved;
    }

    fn visit_impl_item_fn(&mut self, function: &'ast syn::ImplItemFn) {
        let saved = self.const_depth;
        self.const_depth = 0;
        visit::visit_impl_item_fn(self, function);
        self.const_depth = saved;
    }

    fn visit_trait_item_fn(&mut self, function: &'ast syn::TraitItemFn) {
        let saved = self.const_depth;
        self.const_depth = 0;
        visit::visit_trait_item_fn(self, function);
        self.const_depth = saved;
    }

    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        let saved = self.const_depth;
        self.const_depth = 0;
        visit::visit_expr_closure(self, closure);
        self.const_depth = saved;
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        self.const_depth += 1;
        visit::visit_item_static(self, item);
        self.const_depth -= 1;
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        self.const_depth += 1;
        visit::visit_item_const(self, item);
        self.const_depth -= 1;
    }

    fn visit_impl_item_const(&mut self, item: &'ast syn::ImplItemConst) {
        self.const_depth += 1;
        visit::visit_impl_item_const(self, item);
        self.const_depth -= 1;
    }

    fn visit_trait_item_const(&mut self, item: &'ast syn::TraitItemConst) {
        self.const_depth += 1;
        visit::visit_trait_item_const(self, item);
        self.const_depth -= 1;
    }

    fn visit_expr_const(&mut self, expression: &'ast syn::ExprConst) {
        self.const_depth += 1;
        visit::visit_expr_const(self, expression);
        self.const_depth -= 1;
    }
}

fn reject(path: &Path, reason: &str) -> Error {
    Error::Gate(format!("{}: {reason}", path.display()))
}

fn has_setting(source: &str, section: &str, key: &str, value: &str) -> bool {
    let mut active = false;
    for line in source.lines().map(str::trim) {
        if line.starts_with('[') {
            active = line == section;
        } else if active
            && line
                .split_once('=')
                .is_some_and(|(actual_key, actual_value)| {
                    actual_key.trim() == key && actual_value.trim() == value
                })
        {
            return true;
        }
    }
    false
}

const ASYNC_CRATES: &[&str] = &[
    "tokio",
    "async-std",
    "smol",
    "futures",
    "futures-core",
    "futures-util",
    "futures-lite",
    "async-io",
    "async-trait",
    "glommio",
    "monoio",
    "embassy",
    "tonic",
    "reqwest",
    "hyper",
    "axum",
];

fn manifest(path: &Path, source: &str) -> Result<()> {
    let mut dependencies = false;
    for line in source.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            dependencies = line.ends_with("dependencies]");
            if line.contains("dependencies.") {
                return Err(reject(
                    path,
                    "dependency blocks must be single-line inline tables",
                ));
            }
        } else if !line.is_empty() && !line.starts_with('#') {
            if line
                .split_once('=')
                .is_some_and(|(key, value)| key.trim() == "license" && value.trim() != "\"MIT\"")
            {
                return Err(reject(path, "license must be MIT"));
            }
            if dependencies && (!line.contains("= {") || !line.ends_with('}')) {
                return Err(reject(
                    path,
                    "dependency must use a single-line inline table",
                ));
            }
            if dependencies && let Some((name, table)) = line.split_once('=') {
                let package = table
                    .trim()
                    .trim_matches(['{', '}'])
                    .split(',')
                    .find_map(|field| {
                        let (key, value) = field.split_once('=')?;
                        (key.trim().trim_matches(['\"', '\'']) == "package")
                            .then(|| value.trim().trim_matches(['\"', '\'']))
                    });
                if [Some(name.trim().trim_matches(['\"', '\''])), package]
                    .into_iter()
                    .flatten()
                    .any(|name| ASYNC_CRATES.contains(&name))
                {
                    return Err(reject(path, "asynchronous runtime dependency"));
                }
            }
        }
    }
    Ok(())
}

fn rust(path: &Path, source: &str, production: bool) -> Result<()> {
    if source.lines().count() > 800 {
        return Err(reject(path, "file exceeds 800 physical lines"));
    }
    let components: Vec<_> = path.components().collect();
    let has = |component: &str| components.iter().any(|part| part.as_os_str() == component);
    let ui = components
        .windows(2)
        .any(|pair| pair[0].as_os_str() == "tests" && pair[1].as_os_str() == "ui");
    let syntax = syn::parse_file(source)
        .map_err(|error| reject(path, &format!("invalid Rust syntax: {error}")))?;
    let mut audit = RustAudit {
        violations: Violations::default(),
        production,
        ui,
        descriptors: production && !path.ends_with("crates/uarch-core/src/bits/word_macro.rs"),
        patterns: production && !has("uarch-core") && !has("uarch-macros"),
        audited_arch: has("uarch-arch") || has("uarch-probes"),
        tests: has("tests"),
        const_depth: 0,
    };
    audit.violations.chinese = source.chars().any(is_han);
    audit.visit_file(&syntax);
    let violations = audit.violations;
    for (invalid, reason) in [
        (
            violations.descriptor,
            "bit descriptors must be constructed through bitword!",
        ),
        (
            violations.chinese,
            "Chinese text belongs in Markdown documentation, not Rust source",
        ),
        (violations.asynchronous, "asynchronous Rust syntax"),
        (violations.domain, "forbidden domain identifier"),
        (
            violations.runtime_pattern,
            "bit-pattern assembly requires a const context",
        ),
        (violations.extraction, "unchecked production extraction"),
        (
            violations.target_feature,
            "target_feature outside audited architecture kernels",
        ),
        (
            violations.test_name,
            "test function name exceeds 48 characters or 6 segments",
        ),
    ] {
        if invalid {
            return Err(reject(path, reason));
        }
    }
    Ok(())
}

fn mirrors(package: &Path, source: &Path) -> Result<()> {
    let relative = source
        .strip_prefix(package.join("src"))
        .map_err(|error| Error::Gate(error.to_string()))?;
    let entry = relative == Path::new("lib.rs") || relative == Path::new("main.rs");
    for category in ["units", "e2e", "lifecycle", "scenario"] {
        let destination = if entry {
            package.join("tests").join(format!("{category}.rs"))
        } else {
            package.join("tests").join(category).join(relative)
        };
        if !destination.is_file() {
            return Err(reject(&destination, "missing source test mirror"));
        }
    }
    let destination = if entry {
        package.join("benches/main.rs")
    } else {
        package.join("benches").join(relative)
    };
    if !destination.is_file() {
        return Err(reject(&destination, "missing source benchmark mirror"));
    }
    Ok(())
}

#[derive(Deserialize)]
struct Metadata {
    workspace_members: Vec<String>,
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    manifest_path: PathBuf,
    license: Option<String>,
    publish: Option<Vec<String>>,
}

/// Runs an independent structural scan of production Rust sources when ast-grep is installed.
///
/// # Errors
/// Returns an error for forbidden syntax, malformed scanner output, or a scanner execution failure.
pub fn ast_grep_sources(root: &Path) -> Result<()> {
    use std::process::Command;
    match Command::new("ast-grep").arg("--version").output() {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            println!("lint: NotRun: ast-grep unavailable; syn AST checks remain enabled");
            return Ok(());
        }
        Err(error) => return Err(error.into()),
        Ok(output) if !output.status.success() => {
            return Err(Error::Gate("ast-grep version check failed".into()));
        }
        Ok(_) => {}
    }
    let mut sources = Vec::new();
    let entries = match std::fs::read_dir(root.join("crates")) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let source = entry?.path().join("src");
        if source.is_dir() {
            sources.push(source);
        }
    }
    if sources.is_empty() {
        return Err(Error::Gate("ast-grep has no production sources".into()));
    }
    for (id, rule) in [
        (
            "no-async-functions",
            sonic_rs::json!({"kind":"function_item", "has":{"kind":"function_modifiers", "regex":r"\basync\b"}}),
        ),
        ("no-unwrap", sonic_rs::json!({"pattern":"$VALUE.unwrap()"})),
        (
            "no-expect",
            sonic_rs::json!({"pattern":"$VALUE.expect($$$ARGS)"}),
        ),
    ] {
        let config =
            sonic_rs::to_string(&sonic_rs::json!({"id":id,"language":"Rust","rule":rule}))?;
        let output = Command::new("ast-grep")
            .args(["scan", "--inline-rules", &config, "--json=compact"])
            .args(&sources)
            .output()?;
        if !matches!(output.status.code(), Some(0 | 1)) {
            return Err(Error::Gate(format!(
                "ast-grep failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let matches: Vec<sonic_rs::Value> = sonic_rs::from_slice(&output.stdout)?;
        if !matches.is_empty() {
            return Err(Error::Gate(format!(
                "ast-grep {id}: {} production violations",
                matches.len()
            )));
        }
    }
    println!("lint: ast-grep production async/unwrap/expect checks passed");
    Ok(())
}

/// Audits each workspace package using AST matches and deterministic path mirrors.
///
/// # Errors
/// Returns an error for a structural violation, invalid syntax, or unreadable input.
pub fn check_workspace(root: &Path) -> Result<()> {
    manifest(
        &root.join("Cargo.toml"),
        &std::fs::read_to_string(root.join("Cargo.toml"))?,
    )?;
    let metadata: Metadata = sonic_rs::from_slice(
        &process::output(process::cargo(process::PINNED, root).args([
            "metadata",
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
        ]))?
        .stdout,
    )?;
    let root_source = std::fs::read_to_string(root.join("Cargo.toml"))?;
    if !has_setting(&root_source, "[workspace.package]", "license", "\"MIT\"") {
        return Err(reject(root, "workspace license must be MIT"));
    }
    for package in metadata
        .packages
        .iter()
        .filter(|package| metadata.workspace_members.contains(&package.id))
    {
        let package_source = std::fs::read_to_string(&package.manifest_path)?;
        if package
            .publish
            .as_ref()
            .is_none_or(|registries| !registries.is_empty())
            && (package.license.as_deref() != Some("MIT")
                || !has_setting(&package_source, "[package]", "license.workspace", "true"))
        {
            return Err(reject(
                &package.manifest_path,
                "publishable crates must inherit MIT",
            ));
        }
        let directory = package
            .manifest_path
            .parent()
            .ok_or_else(|| reject(&package.manifest_path, "missing package directory"))?;
        manifest(
            &package.manifest_path,
            &std::fs::read_to_string(&package.manifest_path)?,
        )?;
        let mut files = Vec::new();
        files_under(directory, &mut files)?;
        files.sort();
        for file in files {
            if file.extension().is_some_and(|extension| extension == "rs") {
                let source = std::fs::read_to_string(&file)?;
                let production = file.starts_with(directory.join("src"));
                rust(&file, &source, production)?;
                if production {
                    mirrors(directory, &file)?;
                }
            }
        }
    }
    ast_grep_sources(root)?;
    println!("lint: AST, manifest, line-count, and module mirror checks passed");
    Ok(())
}

/// Check library packages and all their targets for the supported AArch64 target.
///
/// # Errors
/// Returns an error if the target is missing or cross-target Clippy fails.
pub fn check_aarch64(root: &Path) -> Result<()> {
    process::run(process::cargo(process::PINNED, root).args([
        "clippy",
        "--workspace",
        "--exclude",
        "xtask",
        "--exclude",
        "uarch-probes",
        "--target",
        "aarch64-unknown-linux-gnu",
        "--all-targets",
        "--locked",
        "--",
        "-D",
        "warnings",
    ]))
}
