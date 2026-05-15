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
    // Fix ! ( → !( (macro invocation spacing, e.g., assert!(...), println!(...))
    s = s.replace("! (", "!(");
    // Also fix space before !( when preceded by identifier: `assert !(` → `assert!(`
    s = fix_macro_bang(&s);
    // Fix # [ → #[ (attribute spacing from TokenStream)
    s = s.replace("# [", "#[");
    // Fix turbofish spacing: `::< X >` → `::<X>`
    // TokenStream::to_string() renders generics with surrounding spaces
    s = fix_turbofish(&s);
    // Fix generic angle spacing: `Box < T >` → `Box<T>` (non-turbofish generics)
    s = fix_generic_angles(&s);
    // Fix trailing space before > in non-turbofish generics (already handled by above)
    // But also fix: `Void >` → `Void>` where not preceded by ::<
    // This catches leftovers after fix_generic_angles processes the open bracket
    s = fix_trailing_gt_spaces(&s);

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
    let s = s.replace("& '", "&'");  // &'lifetime
    
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
        // Skip colons inside doc comments or URL-like patterns
        // Check if the line contains /// (doc comment) or https: (URL)
        let line_start = result.rfind('\n').map(|n| n + 1).unwrap_or(0);
        let line_prefix = &result[line_start..];
        if line_prefix.contains("///") || line_prefix.contains("//!") || line_prefix.contains("https") {
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

/// Fix turbofish spacing: `::< X >` → `::<X>` (and similar generic patterns).
/// TokenStream::to_string() renders generics with surrounding spaces.
fn fix_turbofish(s: &str) -> String {
    let mut result = String::new();
    let mut remaining = s;
    while let Some(pos) = remaining.find("::<") {
        result.push_str(&remaining[..pos]);
        result.push_str("::<");
        let after = &remaining[pos + 3..];
        // Skip any space after <
        let inner = after.trim_start_matches(' ');
        // Find the closing >
        if let Some(close) = inner.find('>') {
            // Get the content between < and >, strip spaces
            let content = &inner[..close];
            let trimmed = content.trim();
            // Remove internal spaces around commas in type lists
            let cleaned_content: String = trimmed
                .split(',')
                .map(|part| part.trim())
                .collect::<Vec<_>>()
                .join(", ");
            result.push_str(&cleaned_content);
            result.push('>');
            remaining = &inner[close + 1..];
        } else {
            // No closing > found, just use as-is
            remaining = inner;
        }
    }
    result.push_str(remaining);
    result
}

/// Fix generic angle bracket spacing: `Box < T >` → `Box<T>` (non-turbofish generics).
/// Only applies when preceded by an identifier (type name), not comparison operators.
fn fix_generic_angles(s: &str) -> String {
    let mut result = String::new();
    let mut remaining = s;
    // Match pattern: identifier < Type >  where Type is simple (no nested <>)
    while let Some(pos) = remaining.find(" < ") {
        result.push_str(&remaining[..pos]);
        // Check if preceded by an identifier character
        if pos > 0 {
            let prev = remaining.as_bytes()[pos - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' || prev == b'>' || prev == b')' {
                // This might be a generic start. Look ahead for >
                let after_space = &remaining[pos + 3..];
                let content_end = after_space.find('>').unwrap_or(0);
                if content_end > 0 {
                    let content = &after_space[..content_end];
                    // Only collapse if the content is fairly simple
                    // (identifier, optional lifetime, commas, nested <>
                    // with matching depth)
                    // For now: only handle single identifier or simple list
                    let trimmed = content.trim();
                    let has_matching_lt = trimmed.matches('<').count() == trimmed.matches('>').count();
                    if has_matching_lt {
                        // Collapse spaces in the content
                        let cleaned: String = trimmed
                            .split(',')
                            .map(|p| {
                                let p = p.trim();
                                // Clean up spaces like `& 'a` → `&'a`
                                let p = p.replace("& ", "&");
                                let p = p.replace("* ", "*");
                                p
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        result.push('<');
                        result.push_str(&cleaned);
                        result.push('>');
                        remaining = &after_space[content_end + 1..];
                        continue;
                    }
                }
            }
        }
        // Not a generic pattern, keep as-is
        result.push_str(" < ");
        remaining = &remaining[pos + 3..];
    }
    result.push_str(remaining);
    result
}

/// Fix trailing space before >: `Void >` → `Void>` in post-generic contexts.
/// This catches leftovers after fix_generic_angles/fix_turbofish process the open bracket.
/// Only applies where > follows a type-like identifier or >.
fn fix_trailing_gt_spaces(s: &str) -> String {
    let mut result = String::new();
    let mut remaining = s;
    while let Some(pos) = remaining.find(" >") {
        result.push_str(&remaining[..pos]);
        // Check if the space is between > and non-space or at end of line
        // Look at what comes before the space
        if pos > 0 {
            let prev = remaining.as_bytes()[pos - 1];
            // Only collapse if preceded by alphanumeric, ), ], or another >
            if prev.is_ascii_alphanumeric() || prev == b')' || prev == b']' {
                // Also check what follows
                let after = &remaining[pos + 2..];
                // Only collapse if followed by something that's NOT an operator
                // (to avoid breaking `a > b` comparisons)
                let next = after.chars().next();
                match next {
                    Some(c) if c.is_alphanumeric() || c == '_' || c == '(' || c == '{' || c == '[' => {
                        // Preceded by type-like, followed by expression-start → likely generic closing
                        result.push('>');
                        remaining = after;
                        continue;
                    }
                    Some('>') => {
                        // `> >` → `>>` (nested generic closing)
                        result.push('>');
                        remaining = after;
                        continue;
                    }
                    Some(',') => {
                        // `> ,` → `>,` (generic closing before comma)
                        result.push('>');
                        remaining = after;
                        continue;
                    }
                    _ => {
                        // Comparison operator or other, keep space
                        result.push_str(" >");
                        remaining = &remaining[pos + 2..];
                        continue;
                    }
                }
            }
        }
        result.push_str(" >");
        remaining = &remaining[pos + 2..];
    }
    result.push_str(remaining);
    result
}

/// Fix space before `!(`: `assert !(` → `assert!(`.
/// TokenStream renders macro calls as `identifier ! (...)`.
fn fix_macro_bang(s: &str) -> String {
    let mut result = String::new();
    let mut remaining = s;
    while let Some(pos) = remaining.find("!(") {
        result.push_str(&remaining[..pos]);
        // Strip trailing space from result (space between macro name and !)
        if result.ends_with(' ') {
            result.pop();
        }
        result.push_str("!(");
        remaining = &remaining[pos + 2..];
    }
    result.push_str(remaining);
    result
}
