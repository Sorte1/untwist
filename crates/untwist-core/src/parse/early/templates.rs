use oxc_diagnostics::OxcDiagnostic;

const PARSER: [&str; 101] = [
    "'X' can only be used with function calls or in property accesses",
    "'X' declaration can only be used at the top level of a module",
    "'X' modifier cannot be used here.",
    "A 'X' accessor cannot have rest parameter.",
    "A 'X' accessor must have exactly one parameter.",
    "A 'X' accessor must not have any formal parameters.",
    "A 'X' expression is only allowed in a generator body.",
    "A 'X' statement can only be used within a function body.",
    "A 'X' statement cannot be used inside a class static block.",
    "A module cannot have multiple default exports.",
    "A reserved word cannot be used as an exported binding without `X`",
    "A rest parameter must be last in a parameter list",
    "A rest parameter or binding pattern may not have a trailing comma.",
    "A unary expression with the 'X' operator cannot be used as the left operand of an exponentiation expression",
    "An export name cannot include a unicode lone surrogate",
    "Argument of dynamic import cannot be a spread element.",
    "Async functions can only be declared at the top level or inside a block",
    "Bad escape sequence in untagged template literal",
    "Cannot assign to this expression",
    "Cannot use `X` as an identifier in a generator context",
    "Cannot use `X` as an identifier in an async context",
    "Cannot use export statement outside a module",
    "Cannot use import statement outside a module",
    "Cannot use new with dynamic import",
    "Classes can only extend a single class.",
    "Classes can'X'#constructor'",
    "Classes can'X'constructor'",
    "Classes may not have a static property named 'X'",
    "Constructor can't be a generator",
    "Constructor can't be an async method",
    "Constructor can't have get/set modifier",
    "Duplicate import attribute key `X`",
    "Duplicated export 'X'",
    "Dynamic imports can only accept a module specifier and an optional set of attributes as arguments",
    "Empty parenthesized expression",
    "Expected `X` but found `X`",
    "Expected `X` or `X` but found `X`",
    "Expected a semicolon or an implicit semicolon after a statement, but found none",
    "Expected function name",
    "Generators can only be declared at the top level or inside a block",
    "Identifier expected.",
    "Identifier expected. 'X' is a reserved word that cannot be used here.",
    "Illegal newline after throw",
    "Invalid Character `X`",
    "Invalid Unicode escape sequence",
    "Invalid assignment in object literal",
    "Invalid characters after number",
    "Invalid class declaration",
    "Invalid escape sequence",
    "Invalid left-hand side in assignment",
    "Invalid optional chain from new expression.",
    "Invalid regular expression: Character class atom range out of order",
    "Invalid regular expression: Character class range with invalid atom",
    "Invalid regular expression: Could not parse the entire pattern",
    "Invalid regular expression: Duplicated capturing group names",
    "Invalid regular expression: Expected nonempty class set expression",
    "Invalid regular expression: Invalid braced quantifier",
    "Invalid regular expression: Invalid character class with strings unicode property",
    "Invalid regular expression: Invalid class atom",
    "Invalid regular expression: Invalid escape sequence",
    "Invalid regular expression: Invalid indexed reference",
    "Invalid regular expression: Invalid named reference",
    "Invalid regular expression: Invalid unicode escape sequence",
    "Invalid regular expression: Invalid unicode flags combination `X` and `X`",
    "Invalid regular expression: Invalid unicode property `X`",
    "Invalid regular expression: Invalid unicode property name",
    "Invalid regular expression: Invalid unicode property name and/or value",
    "Invalid regular expression: Lone quantifier found, expected with `X`",
    "Invalid regular expression: Numbers out of order in braced quantifier",
    "Invalid regular expression: Unexpected `X` inside of class intersection",
    "Invalid regular expression: Unterminated capturing group",
    "Invalid regular expression: Unterminated capturing group name",
    "Invalid regular expression: Unterminated unicode property escape",
    "Invalid rest element target in destructuring assignment",
    "Keywords cannot contain escape characters",
    "Lexical declaration cannot appear in a single-statement context",
    "Line terminator not permitted before arrow",
    "Logical expressions and coalesce expressions cannot be mixed",
    "Missing initializer in const declaration",
    "Missing initializer in destructuring declaration",
    "Only a single declaration is allowed in a `X` statement",
    "Only string literals are allowed as module attribute values.",
    "Spread must be last element",
    "Tagged template expressions are not permitted in an optional chain",
    "The 'X' and 'X' regular expression flags cannot be enabled at the same time",
    "The left-hand side of a `X` statement may not be `X`",
    "The left-hand side of a `X` statement may not start with `X`",
    "The only valid property accesses on import are `X`, `X`, and `X`",
    "Unexpected JSX expression",
    "Unexpected import.meta expression",
    "Unexpected new.target expression",
    "Unexpected private identifier",
    "Unexpected token",
    "Unterminated multiline comment",
    "Unterminated regular expression",
    "Unterminated string",
    "`X` is only allowed within async functions and at the top levels of modules",
    "`X` loops are only allowed within async functions and at the top levels of modules",
    "await can only be used in conjunction with `X` statements",
    "for-in loop variable declaration may not have an initializer",
    "import() requires a specifier.",
];

const SEMANTIC: [&str; 35] = [
    "'X' can only be referenced in a derived class.",
    "'X' can only be referenced in members of derived classes or object literal expressions.",
    "'X' declarations are not allowed at the top level of a script",
    "'X' is not allowed in class field initializer",
    "'X' is not allowed in static initialization block",
    "'X' statements are not allowed",
    "'X'-prefixed octal literals and octal escape sequences are deprecated",
    "A `X` statement can only jump to a label of an enclosing `X`, `X` or `X` statement.",
    "A class name is required.",
    "Cannot assign to 'X' in strict mode",
    "Cannot use 'X' in class static initialization block",
    "Cannot use await in class static initialization block",
    "Decimals with leading zeros are not allowed in strict mode",
    "Delete of an unqualified identifier in strict mode.",
    "Export 'X' is not defined",
    "Identifier `X` has already been declared",
    "Illegal 'X' directive in function with non-simple parameter list",
    "Illegal break statement",
    "Illegal continue statement: no surrounding iteration statement",
    "Invalid escape sequence",
    "Invalid function declaration",
    "Jump target cannot cross function boundary.",
    "Label `X` has already been declared",
    "Multiple constructor implementations are not allowed.",
    "Private field 'X' must be declared in an enclosing class",
    "Private identifier 'X' is not allowed outside class bodies",
    "Super calls are not permitted outside constructors or in nested functions inside constructors.",
    "The keyword 'X' is reserved",
    "The operand of a 'X' operator cannot be a private identifier.",
    "Use of undefined label",
    "`X` cannot be declared as a variable name inside of a `X` declaration",
    "await expression not allowed in formal parameter",
    "for-in loop variable declaration may not have an initializer",
    "for-of loop variable declaration may not have an initializer",
    "yield expression not allowed in formal parameter",
];

const FLAG_MESSAGES: [(&str, &str); 2] = [
    ("Flag ", " is mentioned twice in regular expression literal"),
    ("Unexpected flag ", " in regular expression literal"),
];

fn masked(message: &str, quote: char) -> String {
    let mut out = String::with_capacity(message.len());
    let mut rest = message;
    while let Some(open) = rest.find(quote) {
        let after = &rest[open + quote.len_utf8()..];
        let Some(close) = after.find(quote) else {
            break;
        };
        out.push_str(&rest[..open]);
        out.push(quote);
        out.push('X');
        out.push(quote);
        rest = &after[close + quote.len_utf8()..];
    }
    out.push_str(rest);
    out
}

pub fn template(message: &str) -> String {
    masked(&masked(message, '`'), '\'')
}

fn listed(list: &[&str], diagnostic: &OxcDiagnostic) -> bool {
    list.binary_search(&template(&diagnostic.message).as_str())
        .is_ok()
}

pub fn parser_listed(diagnostic: &OxcDiagnostic) -> bool {
    let message = diagnostic.message.as_ref();
    listed(&PARSER, diagnostic)
        || FLAG_MESSAGES
            .iter()
            .any(|(prefix, suffix)| message.starts_with(prefix) && message.ends_with(suffix))
}

pub fn semantic_listed(diagnostic: &OxcDiagnostic) -> bool {
    listed(&SEMANTIC, diagnostic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lists_are_sorted_for_binary_search() {
        assert!(PARSER.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(SEMANTIC.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn a_template_masks_quoted_and_backticked_names() {
        assert_eq!(
            template("Identifier `g` has already been declared"),
            "Identifier `X` has already been declared"
        );
        assert_eq!(
            template("Cannot assign to 'eval' in strict mode"),
            "Cannot assign to 'X' in strict mode"
        );
        assert_eq!(template("Unterminated string"), "Unterminated string");
    }
}
