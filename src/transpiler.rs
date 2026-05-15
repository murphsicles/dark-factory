// src/transpiler.rs — Core Rust AST → Zeta conversion.
//
// Walks a syn::File (Rust parsed AST) and emits Zeta source text.
// Each node type has a corresponding emit function.
//
// Design: single pass, no intermediate IR. Rewrites are applied inline
// during the walk. The result is a String of valid Zeta code.

use syn::{
    File, Item, FnArg, Pat, Type, Generics, GenericParam, WhereClause,
    Attribute, Visibility, ReturnType, Expr, Stmt,
    Block, PatType, TypeParamBound,
    ItemFn, ItemStruct, ItemEnum, ItemImpl, ItemTrait, ItemUse, ItemMod,
    ItemConst, ItemStatic, ItemType,
    punctuated::Punctuated, token::Plus,
};
use proc_macro2::TokenStream;
use quote::ToTokens;
use std::collections::HashSet;

/// Convert an entire syn::File (Rust module) to Zeta source.
pub fn convert_file(source: &str, filename: &str) -> anyhow::Result<String> {
    let syntax: File = syn::parse_file(source)?;
    let mut ctx = Context::new(filename);
    emit_file(&syntax, &mut ctx);
    let result = crate::post_process::clean(&ctx.output);
    Ok(result)
}

struct Context {
    output: String,
    filename: String,
    indent: usize,
    known_renames: HashSet<String>,
    current_module: Vec<String>,
}

impl Context {
    fn new(filename: &str) -> Self {
        let mut known = HashSet::new();
        known.insert("Option".into());
        known.insert("Result".into());
        known.insert("String".into());
        known.insert("Vec".into());
        known.insert("Box".into());
        known.insert("Arc".into());
        known.insert("Mutex".into());
        known.insert("HashMap".into());
        known.insert("HashSet".into());
        known.insert("PathBuf".into());
        known.insert("Path".into());
        known.insert("Duration".into());
        known.insert("Ordering".into());
        known.insert("AtomicUsize".into());
        known.insert("AtomicBool".into());
        known.insert("AtomicPtr".into());
        Context { output: String::new(), filename: filename.to_string(), indent: 0, known_renames: known, current_module: Vec::new() }
    }

    fn emit(&mut self, s: &str) { self.output.push_str(s); }
    fn emit_line(&mut self, s: &str) {
        for _ in 0..self.indent { self.output.push_str("    "); }
        self.output.push_str(s);
        self.output.push('\n');
    }
    fn push_indent(&mut self) { self.indent += 1; }
    fn pop_indent(&mut self) { if self.indent > 0 { self.indent -= 1; } }
}

fn emit_file(file: &File, ctx: &mut Context) {
    ctx.emit_line(&format!("// Auto-converted from {}", ctx.filename));
    ctx.emit_line("");
    for item in &file.items { emit_item(item, ctx); }
}

fn emit_item(item: &Item, ctx: &mut Context) {
    match item {
        Item::Fn(f) => emit_fn(f, ctx),
        Item::Struct(s) => emit_struct(s, ctx),
        Item::Enum(e) => emit_enum(e, ctx),
        Item::Impl(i) => emit_impl(i, ctx),
        Item::Trait(t) => emit_trait(t, ctx),  // trait stays as "trait" in raw output
        Item::Use(u) => emit_use(u, ctx),
        Item::Mod(m) => emit_mod(m, ctx),
        Item::Const(c) => emit_const(c, ctx),
        Item::Static(s) => emit_static(s, ctx),
        Item::Type(t) => emit_type_alias(t, ctx),
        _ => emit_attrs_only(item, ctx),
    }
}

fn emit_attrs_only(item: &Item, ctx: &mut Context) {
    match item {
        Item::Fn(f) => emit_attrs(&f.attrs, ctx),
        Item::Struct(s) => emit_attrs(&s.attrs, ctx),
        Item::Enum(e) => emit_attrs(&e.attrs, ctx),
        Item::Impl(i) => emit_attrs(&i.attrs, ctx),
        Item::Trait(t) => emit_attrs(&t.attrs, ctx),
        _ => {}
    }
    ctx.emit_line("// [unsupported item]");
}

// ─── Functions ────────────────────────────────────────────────────────────

fn emit_fn(f: &ItemFn, ctx: &mut Context) {
    emit_attrs(&f.attrs, ctx);
    let vis = if matches!(f.vis, Visibility::Public(_)) { "pub " } else { "" };
    let unsf = if f.sig.unsafety.is_some() { "unsafe " } else { "" };
    let const_kw = if f.sig.constness.is_some() { "const " } else { "" };
    let name = &f.sig.ident;
    let (gen_params, gen_where) = params_and_where(&f.sig.generics);
    let params = emit_fn_params(&f.sig.inputs, ctx);
    let ret = emit_return_type(&f.sig.output);

    ctx.emit_line(&format!("{}{}{}fn {}{}({}){}{} {{", vis, const_kw, unsf, name, gen_params, params, ret, gen_where));
    ctx.push_indent();
    emit_block(&f.block, ctx);
    ctx.pop_indent();
    ctx.emit_line("}");
    ctx.emit_line("");
}

fn emit_fn_params(params: &Punctuated<FnArg, syn::Token!(,)>, ctx: &mut Context) -> String {
    let parts: Vec<String> = params.iter().map(|arg| match arg {
        FnArg::Typed(pat) => emit_pat_type(pat, ctx),
        FnArg::Receiver(recv) => {
            if recv.reference.is_some() {
                if recv.mutability.is_some() { "&mut self".into() } else { "&self".into() }
            } else { "self".into() }
        }
    }).collect();
    parts.join(", ")
}

fn emit_pat_type(pat: &PatType, _ctx: &mut Context) -> String {
    let pat_str = pat_ident(&pat.pat);
    let ty_str = emit_type(&pat.ty);
    format!("{}: {}", pat_str, ty_str)
}

fn pat_ident(pat: &Pat) -> String {
    match pat {
        Pat::Ident(p) => p.ident.to_string(),
        Pat::Wild(_) => "_".to_string(),
        _ => "_".to_string(),
    }
}

// ─── Types ────────────────────────────────────────────────────────────────

fn emit_type(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) => {
            let path = &type_path.path;
            let segments: Vec<String> = path.segments.iter().map(|seg| {
                let name = seg.ident.to_string();
                let args = match &seg.arguments {
                    syn::PathArguments::AngleBracketed(args) if !args.args.is_empty() => {
                        let ps: Vec<String> = args.args.iter().map(|a| match a {
                            syn::GenericArgument::Type(t) => emit_type(t),
                            syn::GenericArgument::Lifetime(lt) => lt.to_token_stream().to_string(),
                            syn::GenericArgument::Const(e) => expr_to_string(e),
                            _ => "_".into(),
                        }).collect();
                        format!("<{}>", ps.join(", "))
                    }
                    _ => String::new(),
                };
                format!("{}{}", name, args)
            }).collect();
            segments.join("::")
        }
        Type::Reference(ref_ty) => {
            let mutout = if ref_ty.mutability.is_some() { "mut " } else { "" };
            let inner = emit_type(&ref_ty.elem);
            format!("&{}{}", mutout, inner)
        }
        Type::Slice(slice) => format!("[{}]", emit_type(&slice.elem)),
        Type::Tuple(tuple) => {
            if tuple.elems.is_empty() { "()".into() }
            else { format!("({})", tuple.elems.iter().map(|t| emit_type(t)).collect::<Vec<_>>().join(", ")) }
        }
        Type::Array(arr) => format!("[{}; {}]", emit_type(&arr.elem), expr_to_string(&arr.len)),
        Type::Infer(_) => "_".into(),
        Type::Never(_) => "!".into(),
        Type::Paren(p) => format!("({})", emit_type(&p.elem)),
        Type::Ptr(p) => {
            let mut_kw = if p.mutability.is_some() { "mut " } else { "const " };
            format!("*{}{}", mut_kw, emit_type(&p.elem))
        }
        Type::ImplTrait(t) => {
            // impl Trait — emit bounds with full generic args
            let bounds: Vec<String> = t.bounds.iter().map(|b| {
                match b {
                    TypeParamBound::Trait(tb) => {
                        tb.path.segments.iter().map(|s| {
                            let seg_name = s.ident.to_string();
                            let args = match &s.arguments {
                                syn::PathArguments::AngleBracketed(args) if !args.args.is_empty() => {
                                    let ps: Vec<String> = args.args.iter().map(|a| match a {
                                        syn::GenericArgument::Type(t) => emit_type(t),
                                        syn::GenericArgument::Lifetime(lt) => lt.ident.to_string(),
                                        _ => "_".into(),
                                    }).collect();
                                    format!("<{}>", ps.join(", "))
                                }
                                _ => String::new(),
                            };
                            format!("{}{}", seg_name, args)
                        }).collect::<Vec<_>>().join("::")
                    }
                    TypeParamBound::Lifetime(lt) => lt.ident.to_string(),
                    _ => "_".into(),
                }
            }).collect();
            format!("impl {}", bounds.join(" + "))
        }
        _ => "_".into(),
    }
}

fn emit_generics(generics: &Generics) -> String {
    params_and_where(generics).0
}

/// Return (params_string, where_clause_string) separately.
fn params_and_where(generics: &Generics) -> (String, String) {
    let params_str = if generics.params.is_empty() {
        String::new()
    } else {
        let params: Vec<String> = generics.params.iter().map(|param| match param {
        GenericParam::Type(tp) => {
            let name = &tp.ident;
            if tp.bounds.is_empty() { name.to_string() }
            else {
                let bs: Vec<String> = tp.bounds.iter().map(|b| {
                    match b {
                        TypeParamBound::Trait(t) => {
                            emit_path_with_args(&t.path)
                        }
                        TypeParamBound::Lifetime(lt) => lt.ident.to_string(),
                        _ => "_".into(),
                    }
                }).collect();
                format!("{}: {}", name, bs.join(" + "))
            }
        }
        GenericParam::Lifetime(lt) => format!("{}", lt.lifetime.ident),
        GenericParam::Const(cp) => {
            let ty = emit_type(&cp.ty);
            format!("const {}: {}", cp.ident, ty)
        }
    }).collect();
        format!("<{}>", params.join(", "))
    };
    let where_str = emit_where_clause_str(&generics.where_clause);
    (params_str, where_str)
}

/// Emit generics including where clause (for function signatures).
fn emit_generics_full(generics: &Generics) -> String {
    let (params, wc) = params_and_where(generics);
    format!("{}{}", params, wc)
}

fn emit_where_clause_str(wc: &Option<WhereClause>) -> String {
    match wc {
        Some(wc) if !wc.predicates.is_empty() => {
            let preds: Vec<String> = wc.predicates.iter().map(|p| match p {
                syn::WherePredicate::Type(pt) => {
                    let ty = emit_type(&pt.bounded_ty);
                    let bounds: Vec<String> = pt.bounds.iter().map(|b| match b {
                        TypeParamBound::Trait(tb) => {
                            emit_path_with_args(&tb.path)
                        }
                        TypeParamBound::Lifetime(lt) => lt.ident.to_string(),
                        _ => "_".into(),
                    }).collect();
                    format!("{}: {}", ty, bounds.join(" + "))
                }
                syn::WherePredicate::Lifetime(pl) => {
                    format!("{}: {}", pl.lifetime.ident, pl.bounds.iter().map(|b| b.ident.to_string()).collect::<Vec<_>>().join(" + "))
                }
                _ => "_".into(),
            }).collect();
            format!(" where {}", preds.join(", "))
        }
        _ => String::new(),
    }
}

fn emit_return_type(ret: &ReturnType) -> String {
    match ret {
        ReturnType::Default => String::new(),
        ReturnType::Type(_, ty) => format!(" -> {}", emit_type(ty)),
    }
}

// ─── Structs ──────────────────────────────────────────────────────────────

fn emit_struct(s: &ItemStruct, ctx: &mut Context) {
    emit_attrs(&s.attrs, ctx);
    let vis = if matches!(s.vis, Visibility::Public(_)) { "pub " } else { "" };
    let generics = emit_generics(&s.generics);
    ctx.emit_line(&format!("{}struct {}{} {{", vis, s.ident, generics));
    ctx.push_indent();
    match &s.fields {
        syn::Fields::Named(fields) => {
            for field in &fields.named {
                let fvis = if matches!(field.vis, Visibility::Public(_)) { "pub " } else { "" };
                let name = field.ident.as_ref().unwrap();
                let ty = emit_type(&field.ty);
                ctx.emit_line(&format!("{}{}: {},", fvis, name, ty));
            }
        }
        syn::Fields::Unnamed(fields) => {
            let types: Vec<String> = fields.unnamed.iter().map(|f| emit_type(&f.ty)).collect();
            ctx.emit_line(&format!("({}),", types.join(", ")));
        }
        syn::Fields::Unit => {}
    }
    ctx.pop_indent();
    ctx.emit_line("}");
    ctx.emit_line("");
}

// ─── Enums ────────────────────────────────────────────────────────────────

fn emit_enum(e: &ItemEnum, ctx: &mut Context) {
    emit_attrs(&e.attrs, ctx);
    let vis = if matches!(e.vis, Visibility::Public(_)) { "pub " } else { "" };
    let generics = emit_generics(&e.generics);
    ctx.emit_line(&format!("{}enum {}{} {{", vis, e.ident, generics));
    ctx.push_indent();
    for variant in &e.variants {
        let name = &variant.ident;
        let fields = match &variant.fields {
            syn::Fields::Named(fields) => {
                let f: Vec<String> = fields.named.iter()
                    .map(|f| format!("{}: {}", f.ident.as_ref().unwrap(), emit_type(&f.ty))).collect();
                format!(" {{ {} }}", f.join(", "))
            }
            syn::Fields::Unnamed(fields) => {
                let f: Vec<String> = fields.unnamed.iter().map(|f| emit_type(&f.ty)).collect();
                format!("({})", f.join(", "))
            }
            syn::Fields::Unit => String::new(),
        };
        ctx.emit_line(&format!("{}{},", name, fields));
    }
    ctx.pop_indent();
    ctx.emit_line("}");
    ctx.emit_line("");
}

// ─── Impl blocks ──────────────────────────────────────────────────────────

fn emit_impl(i: &ItemImpl, ctx: &mut Context) {
    emit_attrs(&i.attrs, ctx);
    let unsf = if i.unsafety.is_some() { "unsafe " } else { "" };
    let ty = emit_type(&i.self_ty);
    let (gen_params, gen_where) = params_and_where(&i.generics);
    let trait_path = i.trait_.as_ref().map(|(_, path, _)| {
        path.segments.iter().map(|s| s.ident.to_string()).collect::<Vec<_>>().join("::")
    });
    match trait_path {
        Some(tn) => {
            ctx.emit_line(&format!("{}impl{} {} for {}{} {{", unsf, gen_params, tn, ty, gen_where));
        }
        None => {
            ctx.emit_line(&format!("{}impl{} {}{} {{", unsf, gen_params, ty, gen_where));
        }
    }
    ctx.push_indent();
    for item in &i.items { emit_impl_item(item, ctx); }
    ctx.pop_indent();
    ctx.emit_line("}");
    ctx.emit_line("");
}

fn emit_impl_item(item: &syn::ImplItem, ctx: &mut Context) {
    match item {
        syn::ImplItem::Fn(f) => {
            let vis = if matches!(f.vis, Visibility::Public(_)) { "pub " } else { "" };
            let unsf = if f.sig.unsafety.is_some() { "unsafe " } else { "" };
            let name = &f.sig.ident;
            let params = emit_fn_params(&f.sig.inputs, ctx);
            let ret = emit_return_type(&f.sig.output);
            ctx.emit_line(&format!("{}fn {}({}){} {{", vis, name, params, ret));
            ctx.push_indent();
            emit_block(&f.block, ctx);
            ctx.pop_indent();
            ctx.emit_line("}");
            ctx.emit_line("");
        }
        syn::ImplItem::Type(t) => { ctx.emit_line(&format!("type {} = {};", t.ident, emit_type(&t.ty))); }
        syn::ImplItem::Const(c) => { ctx.emit_line(&format!("const {}: {} = {};", c.ident, emit_type(&c.ty), expr_to_string(&c.expr))); }
        _ => { ctx.emit_line("// [unsupported impl item]"); }
    }
}

// ─── Traits ───────────────────────────────────────────────────────────────

fn emit_trait(t: &ItemTrait, ctx: &mut Context) {
    emit_attrs(&t.attrs, ctx);
    let vis = if matches!(t.vis, Visibility::Public(_)) { "pub " } else { "" };
    let (gen_params, gen_where) = params_and_where(&t.generics);
    // NOTE: Post-processor should convert "trait" to "concept"
    ctx.emit_line(&format!("{}trait {}{}{} {{", vis, t.ident, gen_params, gen_where));
    ctx.push_indent();
    for item in &t.items {
        match item {
            syn::TraitItem::Fn(f) => {
                let params = emit_fn_params(&f.sig.inputs, ctx);
                let ret = emit_return_type(&f.sig.output);
                let (gen_params, gen_where) = params_and_where(&f.sig.generics);
                ctx.emit_line(&format!("fn {}{}({}){}{};", f.sig.ident, gen_params, params, ret, gen_where));
            }
            syn::TraitItem::Type(t) => { ctx.emit_line(&format!("type {};", t.ident)); }
            syn::TraitItem::Const(c) => { ctx.emit_line(&format!("const {}: {};", c.ident, emit_type(&c.ty))); }
            _ => {}
        }
    }
    ctx.pop_indent();
    ctx.emit_line("}");
    ctx.emit_line("");
}

// ─── Use / Mod / Const / Static / Type ────────────────────────────────────

fn emit_use(u: &ItemUse, ctx: &mut Context) {
    // u.to_token_stream() includes the trailing semicolon, so don't add another
    let s = u.to_token_stream().to_string();
    ctx.emit_line(&s);
}

fn emit_mod(m: &ItemMod, ctx: &mut Context) {
    emit_attrs(&m.attrs, ctx);
    let vis = if matches!(m.vis, Visibility::Public(_)) { "pub " } else { "" };
    match &m.content {
        Some((_, items)) => {
            ctx.emit_line(&format!("{}mod {} {{", vis, m.ident));
            ctx.push_indent();
            for item in items { emit_item(item, ctx); }
            ctx.pop_indent();
            ctx.emit_line("}");
            ctx.emit_line("");
        }
        None => { ctx.emit_line(&format!("{}mod {};", vis, m.ident)); }
    }
}

fn emit_const(c: &ItemConst, ctx: &mut Context) {
    let vis = if matches!(c.vis, Visibility::Public(_)) { "pub " } else { "" };
    let ty = emit_type(&c.ty);
    ctx.emit_line(&format!("{}const {}: {} = {};", vis, c.ident, ty, expr_to_string(&c.expr)));
    ctx.emit_line("");
}

fn emit_static(s: &ItemStatic, ctx: &mut Context) {
    let vis = if matches!(s.vis, Visibility::Public(_)) { "pub " } else { "" };
    ctx.emit_line(&format!("{}static {}: {} = {};", vis, s.ident, emit_type(&s.ty), expr_to_string(&s.expr)));
    ctx.emit_line("");
}

fn emit_type_alias(t: &ItemType, ctx: &mut Context) {
    let vis = if matches!(t.vis, Visibility::Public(_)) { "pub " } else { "" };
    let generics = emit_generics(&t.generics);
    ctx.emit_line(&format!("{}type {}{} = {};", vis, t.ident, generics, emit_type(&t.ty)));
    ctx.emit_line("");
}

// ─── Block ───────────────────────────────────────────────────────────────

fn emit_block(block: &Block, ctx: &mut Context) {
    for stmt in &block.stmts { emit_stmt(stmt, ctx); }
}

fn emit_stmt(stmt: &Stmt, ctx: &mut Context) {
    match stmt {
        Stmt::Local(local) => {
            let mut_pat = if let Pat::Ident(p) = &local.pat { if p.mutability.is_some() { "mut " } else { "" } } else { "" };
            let pat_str = pat_ident(&local.pat);
            let init_str = local.init.as_ref().map(|i| {
                let val = expr_to_string(&i.expr);
                format!(" = {}", val)
            }).unwrap_or_default();
            ctx.emit_line(&format!("let {}{}{};", mut_pat, pat_str, init_str));
        }
        Stmt::Item(item) => { emit_item(item, ctx); }
        Stmt::Expr(expr, semi) => {
            let s = expr_to_string(expr);
            if !s.is_empty() {
                // Only add semicolon if the source had one (tail expr = no semi)
                let sep = if semi.is_some() { ";" } else { "" };
                ctx.emit_line(&format!("{}{}", s, sep));
            }
        }
        Stmt::Macro(mac) => {
            emit_attrs(&mac.attrs, ctx);
            let path_str = mac.mac.path.segments.iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            // Render macro tokens preserving delimiter style
            let tokens_raw = mac.mac.tokens.to_string();
            let delim_str = match mac.mac.delimiter {
                syn::MacroDelimiter::Paren(_) => format!("({})", tokens_raw),
                syn::MacroDelimiter::Bracket(_) => format!("[{}]", tokens_raw),
                syn::MacroDelimiter::Brace(_) => format!("{{ {} }}", tokens_raw),
            };
            let semi = if mac.semi_token.is_some() { ";" } else { "" };
            ctx.emit_line(&format!("{}!{}{}", path_str, delim_str, semi));
        }
        _ => {}
    }
}

// ─── Expression to string ────────────────────────────────────────────────

fn expr_to_string(expr: &Expr) -> String {
    let ts: TokenStream = quote::quote!(#expr);
    let s = ts.to_string();
    s
}

// ─── Attributes ───────────────────────────────────────────────────────────

fn emit_attrs(attrs: &[Attribute], ctx: &mut Context) {
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let Ok(nv) = attr.meta.require_name_value() {
                if let syn::Expr::Lit(lit) = &nv.value {
                    if let syn::Lit::Str(s) = &lit.lit {
                        ctx.emit_line(&format!("/// {}", s.value()));
                    }
                }
            }
        } else if attr.path().is_ident("allow") || attr.path().is_ident("warn") || attr.path().is_ident("deny") || attr.path().is_ident("forbid") {
            // skip lint attrs
        } else {
            // All other attrs: emit as-is (cfg, path, inline, derive, must_use, etc.)
            let meta = attr.to_token_stream().to_string();
            ctx.emit_line(&meta);
        }
    }
}

// ─── Post-processing ──────────────────────────────────────────────────────
//
// Cleans up formatting artifacts from quote!() rendering:
//   self . value  →  self.value
//   Some (v)      →  Some(v)
//   ;\n}          →  \n}  (remove trailing semicolons before closing braces)

/// Emit a path including generic args: `Deserializer<'de>` or `Into<Id>`.
fn emit_path_with_args(path: &syn::Path) -> String {
    path.segments.iter().map(|s| {
        let name = s.ident.to_string();
        let args = match &s.arguments {
            syn::PathArguments::AngleBracketed(args) if !args.args.is_empty() => {
                let ps: Vec<String> = args.args.iter().map(|a| match a {
                    syn::GenericArgument::Type(t) => emit_type(t),
                    syn::GenericArgument::Lifetime(lt) => lt.ident.to_string(),
                    syn::GenericArgument::Const(e) => expr_to_string(e),
                    _ => "_".into(),
                }).collect();
                format!("<{}>", ps.join(", "))
            }
            _ => String::new(),
        };
        format!("{}{}", name, args)
    }).collect::<Vec<_>>().join("::")
}
