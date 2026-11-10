#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    And,
    Array,
    Arrow,
    Assign,
    Block,
    Call,
    Callchain,
    Comma,
    Cond,
    Destr,
    Elseif,
    Ifelse,
    Exp,
    Fnexpr,
    Function,
    If,
    Jsx,
    Label,
    Member,
    Optchain,
    Nullish,
    Neg,
    New,
    Numplus,
    Object,
    Paren,
    Plus,
    Stmts,
    Strplus,
    Template,
    Typeof,
    Unary,
    While,
    Klass,
    Try,
    Arrowdef,
    Arrowobj,
    Asyncarrow,
    Arrowcomma,
    Openarr,
    Openparen,
    Openobj,
    Opentpl,
    Jsxattr,
    Coverarr,
    Coverobj,
    Parengrp,
    Callarg,
    Ternparen,
    Stmtcond,
    Ternnest,
    Andnest,
    Ornest,
    Seqblock,
    Arrowbody,
    Ifnb,
    Whilenb,
    Fornb,
    Elseifnb,
    Tagged,
    Lines,
}

pub const TABLE_ONE: &[Class] = &[
    Class::And,
    Class::Array,
    Class::Arrow,
    Class::Assign,
    Class::Block,
    Class::Call,
    Class::Callchain,
    Class::Comma,
    Class::Cond,
    Class::Destr,
    Class::Elseif,
    Class::Ifelse,
    Class::Exp,
    Class::Fnexpr,
    Class::Function,
    Class::If,
    Class::Jsx,
    Class::Label,
    Class::Member,
    Class::Optchain,
    Class::Nullish,
    Class::Neg,
    Class::New,
    Class::Numplus,
    Class::Object,
    Class::Paren,
    Class::Plus,
    Class::Stmts,
    Class::Strplus,
    Class::Template,
    Class::Typeof,
    Class::Unary,
    Class::While,
    Class::Klass,
    Class::Try,
    Class::Tagged,
];

pub const TABLE_THREE: &[Class] = &[
    Class::Arrowdef,
    Class::Arrowobj,
    Class::Asyncarrow,
    Class::Arrowcomma,
    Class::Openarr,
    Class::Openparen,
    Class::Openobj,
    Class::Opentpl,
    Class::Jsxattr,
    Class::Coverarr,
    Class::Coverobj,
    Class::Parengrp,
    Class::Callarg,
    Class::Ternparen,
];

pub const LIFTING: &[Class] = &[
    Class::Stmtcond,
    Class::Ternnest,
    Class::Andnest,
    Class::Ornest,
    Class::Seqblock,
    Class::Arrowbody,
    Class::Ifnb,
    Class::Whilenb,
    Class::Fornb,
    Class::Elseifnb,
];

pub const ALL: &[Class] = &[
    Class::And,
    Class::Array,
    Class::Arrow,
    Class::Assign,
    Class::Block,
    Class::Call,
    Class::Callchain,
    Class::Comma,
    Class::Cond,
    Class::Destr,
    Class::Elseif,
    Class::Ifelse,
    Class::Exp,
    Class::Fnexpr,
    Class::Function,
    Class::If,
    Class::Jsx,
    Class::Label,
    Class::Member,
    Class::Optchain,
    Class::Nullish,
    Class::Neg,
    Class::New,
    Class::Numplus,
    Class::Object,
    Class::Paren,
    Class::Plus,
    Class::Stmts,
    Class::Strplus,
    Class::Template,
    Class::Typeof,
    Class::Unary,
    Class::While,
    Class::Klass,
    Class::Try,
    Class::Arrowdef,
    Class::Arrowobj,
    Class::Asyncarrow,
    Class::Arrowcomma,
    Class::Openarr,
    Class::Openparen,
    Class::Openobj,
    Class::Opentpl,
    Class::Jsxattr,
    Class::Coverarr,
    Class::Coverobj,
    Class::Parengrp,
    Class::Callarg,
    Class::Ternparen,
    Class::Stmtcond,
    Class::Ternnest,
    Class::Andnest,
    Class::Ornest,
    Class::Seqblock,
    Class::Arrowbody,
    Class::Ifnb,
    Class::Whilenb,
    Class::Fornb,
    Class::Elseifnb,
    Class::Tagged,
    Class::Lines,
];

struct Shape {
    prefix: &'static str,
    open: &'static str,
    middle: &'static str,
    close: &'static str,
    suffix: &'static str,
}

const fn shape(
    prefix: &'static str,
    open: &'static str,
    middle: &'static str,
    close: &'static str,
    suffix: &'static str,
) -> Shape {
    Shape {
        prefix,
        open,
        middle,
        close,
        suffix,
    }
}

impl Class {
    pub fn name(self) -> &'static str {
        NAMES[self as usize]
    }

    pub fn from_name(name: &str) -> Option<Class> {
        ALL.iter().copied().find(|class| class.name() == name)
    }
}

const NAMES: &[&str] = &[
    "and",
    "array",
    "arrow",
    "assign",
    "block",
    "call",
    "callchain",
    "comma",
    "cond",
    "destr",
    "elseif",
    "ifelse",
    "exp",
    "fnexpr",
    "function",
    "if",
    "jsx",
    "label",
    "member",
    "optchain",
    "nullish",
    "neg",
    "new",
    "numplus",
    "object",
    "paren",
    "plus",
    "stmts",
    "strplus",
    "template",
    "typeof",
    "unary",
    "while",
    "class",
    "try",
    "arrowdef",
    "arrowobj",
    "asyncarrow",
    "arrowcomma",
    "openarr",
    "openparen",
    "openobj",
    "opentpl",
    "jsxattr",
    "coverarr",
    "coverobj",
    "parengrp",
    "callarg",
    "ternparen",
    "stmtcond",
    "ternnest",
    "andnest",
    "ornest",
    "seqblock",
    "arrowbody",
    "ifnb",
    "whilenb",
    "fornb",
    "elseifnb",
    "tagged",
    "lines",
];

fn table_one_shape(class: Class) -> Option<Shape> {
    Some(match class {
        Class::And => shape("x=", "a&&", "a", "", ";"),
        Class::Array => shape("x=", "[", "", "]", ";"),
        Class::Arrow | Class::Arrowbody => shape("x=", "x=>", "1", "", ";"),
        Class::Assign => shape("x=", "a=", "1", "", ";"),
        Class::Block => shape("", "{", "", "}", ""),
        Class::Call => shape("x=", "f(", "", ")", ";"),
        Class::Callchain => shape("x=f", "()", "", "", ";"),
        Class::Comma => shape("x=(", "a,", "a", "", ");"),
        Class::Cond => shape("x=", "a?b:", "c", "", ";"),
        Class::Destr => shape("var ", "[", "a", "]", "=x;"),
        Class::Elseif => shape("if(a)b;", "else if(a)b;", "", "", ""),
        Class::Ifelse => shape("", "if(a)", "b;", "else b;", ""),
        Class::Exp => shape("x=", "a**", "a", "", ";"),
        Class::Fnexpr => shape("x=", "function(){return ", "1", "}", ";"),
        Class::Function => shape("", "function f(){", "", "}", ""),
        Class::If => shape("", "if(a){", "", "}", ""),
        Class::Jsx => shape("x=", "<a>", "", "</a>", ";"),
        Class::Label => shape("", "a:", "b;", "", ""),
        Class::Member => shape("x=a", ".b", "", "", ";"),
        Class::Optchain => shape("x=a", "?.b", "", "", ";"),
        Class::Nullish => shape("x=", "a??", "a", "", ";"),
        Class::Neg => shape("x=", "- ", "a", "", ";"),
        Class::New => shape("x=", "new ", "a", "", ";"),
        Class::Numplus => shape("x=", "1+", "1", "", ";"),
        Class::Object => shape("x=", "{a:", "1", "}", ";"),
        Class::Paren => shape("x=", "(", "1", ")", ";"),
        Class::Plus => shape("x=", "y+", "y", "", ";"),
        Class::Stmts => shape("", "a;", "", "", ""),
        Class::Strplus => shape("x=", "'a'+", "'a'", "", ";"),
        Class::Template => shape("x=", "`${", "1", "}`", ";"),
        Class::Typeof => shape("x=", "typeof ", "a", "", ";"),
        Class::Unary => shape("x=", "!", "a", "", ";"),
        Class::While => shape("", "while(a){", "", "}", ""),
        Class::Klass => shape("x=", "class{m(){return ", "1", "}}", ";"),
        Class::Try => shape("", "try{", "", "}finally{}", ""),
        Class::Tagged => shape("x=a", "``", "", "", ";"),
        _ => return None,
    })
}

fn other_shape(class: Class) -> Shape {
    match class {
        Class::Arrowdef => shape("x=", "(a=", "1", ")", ";"),
        Class::Arrowobj => shape("x=", "({a}=", "1", ")", ";"),
        Class::Asyncarrow => shape("x=", "async(a=", "1", ")", ";"),
        Class::Arrowcomma => shape("x=", "(a,", "a", ")", ";"),
        Class::Openarr => shape("", "[", "", "", ""),
        Class::Openparen => shape("", "(", "", "", ""),
        Class::Openobj => shape("", "{a:", "", "", ""),
        Class::Opentpl => shape("", "`${", "", "", ""),
        Class::Jsxattr => shape("x=", "<a b={", "1", "}/>", ";"),
        Class::Coverarr => shape("x=", "[", "a", "]", "=y;"),
        Class::Coverobj => shape("x=(", "{a:", "b", "}", "=y);"),
        Class::Parengrp => shape("x=", "(", "a", ")", ";"),
        Class::Callarg => shape("x=", "f(a=", "1", ")", ";"),
        Class::Ternparen => shape("x=", "(a?", "b", ":c)", ";"),
        Class::Ternnest => shape("", "a?", "x", ":y", ";"),
        Class::Andnest => shape("", "a&&(", "f()", ")", ";"),
        Class::Ornest => shape("", "a||(", "f()", ")", ";"),
        Class::Seqblock => shape("", "if(a){", "b(),c();", "}", ""),
        Class::Ifnb => shape("", "if(a)", "f();", "", ""),
        Class::Whilenb => shape("", "while(a)", "f();", "", ""),
        Class::Fornb => shape("", "for(;;)", "f();", "", ""),
        Class::Elseifnb => shape("if(a)x;", "else if(a)x;", "", "", ""),
        Class::Lines => shape("", "a = b.c(d)\n", "", "", ""),
        _ => shape("", "", "", "", ""),
    }
}

fn build(shape: &Shape, n: usize) -> String {
    let mut out = String::with_capacity(
        shape.prefix.len()
            + n * (shape.open.len() + shape.close.len())
            + shape.middle.len()
            + shape.suffix.len(),
    );
    out.push_str(shape.prefix);
    for _ in 0..n {
        out.push_str(shape.open);
    }
    out.push_str(shape.middle);
    for _ in 0..n {
        out.push_str(shape.close);
    }
    out.push_str(shape.suffix);
    out
}

pub fn generate(class: Class, n: usize) -> String {
    if class == Class::Stmtcond {
        let chain = "a?b:".repeat(n);
        return format!("{chain}c;function f(){{return {chain}c}}");
    }
    let shape = table_one_shape(class).unwrap_or_else(|| other_shape(class));
    build(&shape, n)
}

pub fn repeatchain(k: usize) -> String {
    let line = format!("{}\n", generate(Class::Arrowobj, 500));
    line.repeat(k)
}
