// src/rewrites.rs — Named rewrite passes for Rust→Zeta conversion.
//
// Each rule is a structural transformation applied during AST walking.

/// A named rewrite rule with a description.
pub struct Rule {
    pub name: &'static str,
    pub description: &'static str,
}

pub const RULES: &[Rule] = &[
    Rule { name: "trait_concept", description: "trait → concept (rename, same syntax)" },
    Rule { name: "fn_syntax", description: "fn name<T: Bound>(params) -> Ret → fn name[T: Bound](params) -> Ret (generics brackets)" },
    Rule { name: "struct_lit", description: "Struct { field: value } → Struct { field: value } (same syntax)" },
    Rule { name: "enum_variant", description: "Enum::Variant(value) → Enum::Variant(value) (same syntax)" },
    Rule { name: "match_patterns", description: "match / if let → same (Zeta has identical pattern matching)" },
    Rule { name: "impl_block", description: "impl Trait for Type → same syntax" },
    Rule { name: "use_statement", description: "use crate::module → use crate::module (same)" },
    Rule { name: "extern_block", description: "extern \"C\" { fn ... } → extern fn ... (Zeta syntax)" },
    Rule { name: "macro_invocation", description: "println!(...), vec![...] → preserve with !! suffix" },
    Rule { name: "unsafe_preserve", description: "unsafe { ... } → unsafe { ... } (same syntax in Zeta)" },
    Rule { name: "lifetime_strip", description: "Remove lifetime annotations where Zeta doesn't need them" },
    Rule { name: "derive_attr", description: "#[derive(Debug, Clone)] → preserve (Zeta has #[derive])" },
    Rule { name: "type_ascription", description: "let x: T = ... → let x: T = ... (same)" },
    Rule { name: "loop_syntax", description: "loop / while / for → same syntax" },
    Rule { name: "closure_syntax", description: "|args| body → same syntax" },
    Rule { name: "self_param", description: "&self / &mut self → same" },
    Rule { name: "result_option", description: "Result<T, E>, Option<T> → same (Zeta stdlib)" },
    Rule { name: "pub_visibility", description: "pub / pub(crate) → pub (Zeta supports pub)" },
];

/// List all available rewrite rules as formatted strings.
pub fn list_rules() -> Vec<String> {
    RULES.iter().map(|r| format!("  {:<30} — {}", r.name, r.description)).collect()
}
