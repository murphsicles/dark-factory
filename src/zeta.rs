// src/zeta.rs — Zeta language specifics: keywords, stdlib mapping, reserved words.

use std::collections::HashMap;

/// Rust keyword → Zeta keyword mapping (where they differ).
pub fn keyword_map() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    // Most Rust keywords are the same in Zeta
    m.insert("fn", "fn");
    m.insert("let", "let");
    m.insert("mut", "mut");
    m.insert("return", "return");
    m.insert("if", "if");
    m.insert("else", "else");
    m.insert("while", "while");
    m.insert("for", "for");
    m.insert("in", "in");
    m.insert("loop", "loop");
    m.insert("match", "match");
    m.insert("struct", "struct");
    m.insert("enum", "enum");
    m.insert("impl", "impl");
    m.insert("trait", "concept");       // KEY DIFFERENCE
    m.insert("use", "use");
    m.insert("mod", "mod");
    m.insert("pub", "pub");
    m.insert("const", "const");
    m.insert("static", "static");
    m.insert("unsafe", "unsafe");
    m.insert("extern", "extern");
    m.insert("self", "self");
    m.insert("super", "super");
    m.insert("crate", "crate");
    m.insert("where", "where");
    m.insert("type", "type");
    m.insert("async", "async");
    m.insert("await", "await");
    m.insert("move", "move");
    m.insert("ref", "ref");
    m.insert("true", "true");
    m.insert("false", "false");
    m
}

/// Rust stdlib path → Zeta equivalent.
pub fn stdlib_path(path: &str) -> Option<&'static str> {
    let m: HashMap<&str, &str> = [
        ("std::option::Option", "std::option::Option"),
        ("std::result::Result", "std::result::Result"),
        ("std::string::String", "std::string::String"),
        ("std::vec::Vec", "std::collections::Vec"),
        ("std::boxed::Box", "std::boxed::Box"),
        ("std::sync::Arc", "std::sync::Arc"),
        ("std::sync::Mutex", "std::sync::Mutex"),
        ("std::collections::HashMap", "std::collections::HashMap"),
        ("std::collections::HashSet", "std::collections::HashSet"),
        ("std::path::PathBuf", "std::path::PathBuf"),
        ("std::path::Path", "std::path::Path"),
        ("std::time::Duration", "std::time::Duration"),
        ("std::sync::atomic::AtomicUsize", "std::sync::atomic::AtomicUsize"),
        ("std::sync::atomic::AtomicBool", "std::sync::atomic::AtomicBool"),
        ("std::sync::atomic::AtomicPtr", "std::sync::atomic::AtomicPtr"),
        ("std::sync::atomic::Ordering", "std::sync::atomic::Ordering"),
        ("std::thread", "std::thread"),
        ("std::io", "std::io"),
        ("std::fmt", "std::fmt"),
        ("std::mem", "std::mem"),
        ("std::ptr", "std::ptr"),
        ("std::env", "std::env"),
        ("std::process", "std::process"),
        ("std::net", "std::net::tcp"),
    ].into_iter().collect();
    m.get(path).copied()
}
