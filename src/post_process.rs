// Dark Factory — Output post-processing.
// Cleans up formatting artifacts from quote!() rendering.

/// Clean up common formatting artifacts in transpiled Zeta output.
pub fn clean(input: &str) -> String {
    let mut s = input.to_string();

    // Fix "self . field" → "self.field"
    s = s.replace("self . ", "self.");
    s = s.replace("Self . ", "Self::");
    s = s.replace(" . ", ".");

    // Fix trailing semicolons before closing braces
    loop {
        let before = s.len();
        s = s.replace("};", "}");
        s = s.replace(";\n}", "\n}");
        if s.len() == before {
            break;
        }
    }

    // Fix " ( " → "(" and " )" → ")"
    s = s.replace(" ( ", "(");
    s = s.replace(" )", ")");
    s = s.replace(" ;", ";");
    s = s.replace(",)", ")");

    // Fix "match & self" → "match &self"
    s = s.replace("& self", "&self");
    s = s.replace("& mut ", "&mut ");

    // Fix method call spacing
    s = s.replace(" ()", "()");
    s = s.replace("true ;", "true;");
    s = s.replace("false ;", "false;");
    s = s.replace("Ok (", "Ok(");
    s = s.replace("Err (", "Err(");
    s = s.replace("None ,", "None,");
    s = s.replace(" , ", ", ");
    s = s.replace("Some (", "Some(");

    // Fix " : " → ": "  (space before colon in field definitions)
    s = s.replace(" : ", ": ");

    // Fix enum variant with comma before tuple: "Some,(T)" → "Some(T),"
    s = s.replace(",(T)", "(T),");  // Named tuple enum variant
    
    // Fix double blank lines
    while s.contains("\n\n\n") {
        s = s.replace("\n\n\n", "\n\n");
    }

    s
}
