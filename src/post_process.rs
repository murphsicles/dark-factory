/// Dark Factory — Output post-processing.
/// Cleans up formatting artifacts from proc_macro2::TokenStream::to_string().
///
/// proc_macro2 renders tokens with spaces around ::, before (, after &/*, etc.
/// This module strips those artifacts.

/// Clean up common formatting artifacts in transpiled Zeta output.
pub fn clean(input: &str) -> String {
    let mut s = input.to_string();

    // ── Phase 1: :: spacing (MUST come before single-colon fixes) ────────
    s = collapse_colons(&s);

    // ── Phase 2: Dot spacing: `word . word` → `word.word` ───────────────
    s = fix_dot_spacing(&s);

    // ── Phase 3: Paren spacing: `fn (args)` → `fn(args)` ────────────────
    s = fix_paren_spacing(&s);

    // ── Phase 4: Reference/deref: `& self` → `&self`, `* mut` → `*mut` ──
    s = fix_ref_spacing(&s);

    // ── Phase 5: Semicolons ──────────────────────────────────────────────
    s = s.replace(" ;", ";");
    // Strip trailing semicolons before close braces
    // Only strip `;}` (semicolon THEN close-brace) not `};` (brace THEN semicolon)
    // which is valid in use-statements-with-braces.
    loop {
        let before = s.len();
        s = s.replace(";}", "}");
        s = s.replace(";\n}", "\n}");
        if s.len() == before { break; }
    }

    // ── Phase 6: Comma spacing ───────────────────────────────────────────
    s = s.replace(" , ", ", ");
    s = s.replace(",)", ")");
    s = s.replace(", }", " }");

    // ── Phase 7: Colon spacing: `value : Type` → `value: Type` ───────────
    s = fix_single_colon(&s);

    // ── Phase 8: Collapse `& *` → `&*` (reborrow pattern) ──────────────
    s = s.replace("& *", "&*");

    // ── Phase 9: Double spaces ───────────────────────────────────────────
    while s.contains("  ") {
        s = s.replace("  ", " ");
    }

    // ── Phase 10: Double blank lines ─────────────────────────────────────
    while s.contains("\n\n\n") {
        s = s.replace("\n\n\n", "\n\n");
    }

    // ── Phase 11: Edge-case polish ────────────────────────────────────────
    s = s.replace(" ;", ";");  // catch any remaining space-semicolons
    // Fix ! ( → !( (macro invocation spacing)
    s = s.replace("! (", "!(");
    // Fix # [ → #[ (attribute spacing from TokenStream)
    s = s.replace("# [", "#[");

    s
}

/// Collapse spaces around `::` without touching single `:`.
fn collapse_colons(s: &str) -> String {
    let mut result = String::new();
    let mut remaining = s;
    while let Some(pos) = remaining.find("::") {
        result.push_str(&remaining[..pos]);
        // Trim trailing space from result (space before ::)
        if result.ends_with(' ') {
            let trimmed = result.trim_end();
            result.truncate(trimmed.len());
        }
        result.push_str("::");
        // Skip past :: and any trailing spaces
        let after = &remaining[pos + 2..];
        remaining = after.trim_start_matches(' ');
    }
    result.push_str(remaining);
    result
}

/// Fix `word . word` → `word.word` (method chaining, field access)
fn fix_dot_spacing(s: &str) -> String {
    let mut result = String::new();
    let mut remaining = s;
    while let Some(pos) = remaining.find(" . ") {
        result.push_str(&remaining[..pos]);
        if pos > 0 {
            let prev = remaining.as_bytes()[pos - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' || prev == b')' || prev == b'>' {
                result.push('.');
            } else {
                result.push_str(" . ");
            }
        } else {
            result.push_str(" . ");
        }
        remaining = &remaining[pos + 3..];
    }
    result.push_str(remaining);
    result
}

/// Fix `fn (args)` → `fn(args)` — space before open paren after identifier.
fn fix_paren_spacing(s: &str) -> String {
    let mut result = String::new();
    let mut remaining = s;
    while let Some(pos) = remaining.find(" (") {
        result.push_str(&remaining[..pos]);
        if pos > 0 {
            let prev = remaining.as_bytes()[pos - 1];
            if prev.is_ascii_alphanumeric() || prev == b'>' || prev == b'_' || prev == b')' || prev == b']' {
                result.push('(');
                remaining = &remaining[pos + 2..];
                continue;
            }
        }
        result.push_str(" (");
        remaining = &remaining[pos + 2..]; // skip both the space and the paren
    }
    result.push_str(remaining);
    result
}

/// Fix `& self` → `&self`, `& mut` → `&mut`, `& *` → `&*`, and `* foo` → `*foo`.
fn fix_ref_spacing(s: &str) -> String {
    // Static replacements
    let s = s.replace("& self", "&self");
    let s = s.replace("& mut ", "&mut ");
    let s = s.replace("& *", "&*");
    
    // Handle & followed by identifier: `& foo` → `&foo`
    // But preserve & followed by bitwise/ref operators: `& &`, `& |`, etc.
    let mut result = String::new();
    let mut remaining = &s[..];
    while let Some(pos) = remaining.find("& ") {
        result.push_str(&remaining[..pos]);
        result.push('&');
        let after = &remaining[pos + 2..];
        let strip = after.starts_with(|c: char| c.is_alphanumeric() || c == '_');
        if strip {
            // Strip space before identifier (reference type or borrow)
        } else {
            result.push(' ');
        }
        remaining = after;
    }
    result.push_str(remaining);
    
    // Handle * followed by identifier (dereference vs multiplication)
    // Dereference: `*ptr` — space should be stripped
    // Multiplication: `n * m` — space should be kept
    // Heuristic: if preceding char is alnum/`)`/`]`, it's multiplication; otherwise dereference
    let mut result2 = String::new();
    remaining = &result[..];
    while let Some(pos) = remaining.find("* ") {
        result2.push_str(&remaining[..pos]);
        result2.push('*');
        let after = &remaining[pos + 2..];
        let is_deref = if pos > 0 {
            // Look backwards past any spaces to find actual preceding non-space char
            let before_space = remaining[..pos].trim_end();
            let prev = before_space.chars().last();
            match prev {
                // Preceded by alphanumeric, ), ], or > → multiplication → keep space
                Some(c) if c.is_alphanumeric() || c == ')' || c == ']' || c == '>' => false,
                _ => true, // dereference
            }
        } else {
            // At start of string → dereference
            true
        };
        if is_deref && after.starts_with(|c: char| c.is_alphanumeric() || c == '_') {
            // Strip space before identifier (dereference)
        } else {
            result2.push(' ');
        }
        remaining = after;
    }
    result2.push_str(remaining);
    result2
}

/// Fix single-colon `:` spacing: `value : Type` → `value: Type`.
/// Does NOT touch `::` (already collapsed in Phase 1).
/// IMPORTANT: after Phase 1 (collapse_colons), `::` appears as bare `::`.
/// We must skip BOTH colons of any `::` pair to avoid splitting them.
fn fix_single_colon(s: &str) -> String {
    let mut result = String::new();
    let mut remaining = s;
    while let Some(pos) = remaining.find(':') {
        result.push_str(&remaining[..pos]);
        let after = &remaining[pos + 1..];
        if after.starts_with(':') {
            // Part of `::` — skip BOTH colons as a unit
            result.push_str("::");
            remaining = &remaining[pos + 2..]; // skip both ::
            continue;
        }
        // Check if this colon is inside a string literal by counting quotes
        let in_string = result.matches('"').count() % 2 == 1;
        if in_string {
            // Inside a string literal — preserve as-is
            result.push(':');
            remaining = after;
            continue;
        }
        // Single colon: strip leading space, add one trailing space
        if result.ends_with(' ') {
            let trimmed = result.trim_end();
            result.truncate(trimmed.len());
        }
        result.push_str(": ");
        remaining = after.trim_start_matches(' ');
    }
    result.push_str(remaining);
    result
}
