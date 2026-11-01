#[derive(Clone, Copy)]
struct Scope {
    awaits: bool,
    yields: bool,
    depth: u32,
}

impl Scope {
    fn inner(self) -> Scope {
        Scope {
            depth: self.depth + 1,
            ..self
        }
    }

    fn function(self, awaits: bool, yields: bool) -> Scope {
        Scope {
            awaits,
            yields,
            depth: self.depth + 1,
        }
    }
}

pub struct Soup {
    state: u64,
    out: String,
}

const MAX_DEPTH: u32 = 4;

const NAMES: &[&str] = &[
    "a", "b", "x", "async", "of", "let", "get", "set", "static", "\u{f1}", "\\u0061b", "a\\u{62}",
    "\u{2135}", "yield", "await", "target", "as", "from",
];

const NUMBERS: &[&str] = &["1", ".5", "0x1F", "1e+5", "1_000", "0b1", "2n", "1.e-3"];

const STRINGS: &[&str] = &[
    "'a'",
    "\"b\\\"c\"",
    "'(['",
    "\"\\u{41}\"",
    "'line\\\ncont'",
    "'\u{2028}'",
    "\"/*\"",
    "'`'",
    "\"<!--\"",
];

const REGEXES: &[&str] = &[
    "/[/(]a\\/(/g",
    "/=x/",
    "/(?:a)/",
    "/[\\]]/",
    "/'/",
    "/\"/",
    "/`/",
    "/\\//i",
];

const SEPARATORS: &[&str] = &[
    " ", " ", " ", " ", "\n", "\r\n", "/* c */", "/*\n*/", "\u{a0}", "\u{feff}", "\u{2028}", "\t",
    "// c\n",
];

const BINARY: &[&str] = &[
    "+",
    "-",
    "*",
    "/",
    "%",
    "**",
    "==",
    "===",
    "!=",
    "<",
    ">",
    "<=",
    ">=",
    "<<",
    ">>",
    ">>>",
    "&",
    "|",
    "^",
    "&&",
    "||",
    "??",
    " in ",
    " instanceof ",
];

const ASSIGN: &[&str] = &["=", "+=", "/=", "-=", "**=", "&&=", "??="];

const MEMBERS: &[&str] = &[
    "b", "if", "class", "await", "yield", "return", "in", "function", "of", "new", "typeof",
];

const JSX_TEXT: &[&str] = &["text", "a / b", "(", "[[", "'", "\"", "x <!-- y", "}"];

impl Soup {
    pub fn new(seed: u64) -> Soup {
        Soup {
            state: seed | 1,
            out: String::new(),
        }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }

    fn pick(&mut self, items: &[&str]) {
        let item = items[self.below(items.len())];
        self.out.push_str(item);
    }

    fn word(&mut self, text: &str) {
        self.out.push_str(text);
    }

    fn gap(&mut self) {
        if self.chance(80) {
            self.out.push(' ');
        } else {
            self.pick(SEPARATORS);
        }
    }

    pub fn program(&mut self) -> String {
        self.out.clear();
        if self.chance(3) {
            self.word("#!/usr/bin/env node\n");
        }
        let top = Scope {
            awaits: self.chance(30),
            yields: false,
            depth: 0,
        };
        for _ in 0..1 + self.below(5) {
            self.statement(top);
            self.gap();
        }
        std::mem::take(&mut self.out)
    }

    fn statement(&mut self, scope: Scope) {
        if scope.depth >= MAX_DEPTH {
            self.expression(scope);
            self.word(";");
            return;
        }
        match self.below(25) {
            0..=4 => self.expression_statement(scope),
            5 => self.if_statement(scope),
            6 => self.loop_statement(scope),
            7 => self.for_statement(scope),
            8 => self.block(scope),
            9 => self.declaration(scope),
            10 => self.function(scope, true),
            11 => {
                if self.chance(30) {
                    self.pick(&["@a ", "@a.b ", "@(a) ", "@a() "]);
                }
                self.class(scope);
            }
            12 => self.return_statement(scope),
            13 => self.labelled(scope),
            14 => self.try_statement(scope),
            15 => self.switch_statement(scope),
            16 => self.html_comment(scope),
            17 => self.asi_block(scope),
            18 => self.brace_then_regex(scope),
            19 => self.word(";"),
            20 => self.module_item(scope),
            21 => self.head_then_regex(scope),
            22 => self.jump_then_regex(scope),
            _ => self.expression_statement(scope),
        }
    }

    fn expression_statement(&mut self, scope: Scope) {
        self.expression(scope);
        if self.chance(70) {
            self.word(";");
        } else {
            self.word("\n");
        }
    }

    fn if_statement(&mut self, scope: Scope) {
        self.word("if (");
        self.expression(scope.inner());
        self.word(")");
        self.gap();
        self.statement(scope.inner());
        if self.chance(50) {
            self.gap();
            self.word("else");
            let text = if self.chance(50) { " " } else { "\n" };
            self.word(text);
            self.statement(scope.inner());
        }
    }

    fn loop_statement(&mut self, scope: Scope) {
        if self.chance(50) {
            self.word("while (");
            self.expression(scope.inner());
            self.word(")");
            self.gap();
            self.statement(scope.inner());
        } else {
            self.word("do ");
            self.statement(scope.inner());
            self.word(" while (");
            self.expression(scope.inner());
            self.word(")");
        }
    }

    fn for_statement(&mut self, scope: Scope) {
        match self.below(8) {
            0 => {
                self.word("for (");
                self.expression(scope.inner());
                self.word("; ");
                self.expression(scope.inner());
                self.word(";)");
            }
            1 => self.word("for (x of "),
            2 => self.word("for (let x in "),
            3 => self.word("for (x of\n{} / "),
            4 => self.word("for (let of of "),
            5 => self.word("for (var of of "),
            6 => self.word("for (const of of "),
            _ => self.word("for await (const x of "),
        }
        if !self.out.ends_with(')') {
            self.expression(scope.inner());
            self.word(")");
        }
        self.gap();
        self.statement(scope.inner());
    }

    fn block(&mut self, scope: Scope) {
        self.word("{");
        for _ in 0..self.below(3) {
            self.gap();
            self.statement(scope.inner());
        }
        self.word("}");
    }

    fn declaration(&mut self, scope: Scope) {
        self.pick(&["var ", "let ", "const "]);
        self.pick(&["x", "y", "[a, b]", "{a, b: c}"]);
        self.word(" = ");
        self.expression(scope.inner());
        self.word(";");
    }

    fn function(&mut self, scope: Scope, declaration: bool) {
        let awaits = self.chance(40);
        let yields = self.chance(30);
        if awaits {
            self.word("async ");
        }
        self.word("function");
        if yields {
            self.word("*");
        }
        if declaration || self.chance(50) {
            self.word(" f");
        }
        self.parameters(scope);
        self.body(scope.function(awaits, yields));
    }

    fn parameters(&mut self, scope: Scope) {
        self.word("(");
        if self.chance(50) {
            self.word("p, q = ");
            self.operand(scope.inner());
        }
        self.word(")");
    }

    fn body(&mut self, scope: Scope) {
        self.word(" {");
        for _ in 0..self.below(4) {
            self.gap();
            self.statement(scope);
        }
        self.word("}");
    }

    fn class(&mut self, scope: Scope) {
        self.word("class");
        if self.chance(70) {
            self.word(" A");
        }
        if self.chance(30) {
            self.word(" extends ");
            self.operand(scope.inner());
        }
        self.word(" {");
        for _ in 0..self.below(4) {
            self.gap();
            self.class_member(scope.inner());
        }
        self.word("}");
    }

    fn class_member(&mut self, scope: Scope) {
        match self.below(7) {
            2 => {
                let member = crate::depth_names::soup_member(self.next());
                self.word(&member);
            }
            0 => {
                self.pick(&["x", "#p", "static y", "'s'", "[k]", "await", "yield"]);
                if self.chance(70) {
                    self.word(" = ");
                    self.expression(scope);
                }
                let text = if self.chance(60) { ";" } else { "\n" };
                self.word(text);
            }
            1 => {
                self.word("static {");
                self.gap();
                self.statement(scope.function(true, false));
                self.word("}");
            }
            _ => self.method(scope),
        }
    }

    fn method(&mut self, scope: Scope) {
        let awaits = self.chance(40);
        let yields = self.chance(30);
        if self.chance(20) {
            self.word("static ");
        }
        match self.below(4) {
            0 => self.word("get "),
            1 => self.word("set "),
            _ => {}
        }
        if awaits {
            self.word("async ");
        }
        if yields {
            self.word("*");
        }
        self.pick(&[
            "m", "if", "class", "'s'", "1", "[k]", "#q", "get", "async", "await", "as", "[\u{e9}]",
        ]);
        self.parameters(scope);
        self.body(scope.function(awaits, yields));
    }

    fn return_statement(&mut self, scope: Scope) {
        self.word("return");
        if self.chance(70) {
            self.gap();
            self.expression(scope.inner());
        }
        self.word(";");
    }

    fn labelled(&mut self, scope: Scope) {
        self.pick(&["l: ", "async: ", "of: "]);
        self.statement(scope.inner());
    }

    fn try_statement(&mut self, scope: Scope) {
        self.word("try ");
        self.block(scope);
        if self.chance(70) {
            self.word(" catch (e) ");
            self.block(scope);
        }
        self.word(" finally ");
        self.block(scope);
    }

    fn switch_statement(&mut self, scope: Scope) {
        self.word("switch (");
        self.expression(scope.inner());
        self.word(") { case ");
        self.expression(scope.inner());
        self.word(": ");
        self.statement(scope.inner());
        self.word(" default: ");
        self.statement(scope.inner());
        self.word("}");
    }

    fn html_comment(&mut self, scope: Scope) {
        match self.below(4) {
            0 => self.word("\n<!-- (( ` text\n"),
            1 => self.word("\n--> (( ` text\n"),
            2 => {
                self.word("a <!-- ");
                self.pick(&["b", "` ((", "b\n"]);
                self.word("\n");
            }
            _ => {
                self.expression(scope.inner());
                self.word("\n--> b\n");
            }
        }
    }

    fn module_item(&mut self, scope: Scope) {
        match self.below(4) {
            0 => self.word("export default "),
            1 => self.word("export "),
            2 => {
                self.pick(&[
                    "import x from 'm'",
                    "export * from 'm'",
                    "export {a} from 'm'",
                    "import x from 'm' assert {type: 'json'}",
                    "import 'm' assert {}",
                    "export * from 'm' assert {}",
                    "export {a} from 'm' with {type: 'json'}",
                ]);
                let end = if self.chance(50) { ";" } else { "\n" };
                self.word(end);
                self.pick(REGEXES);
                return;
            }
            _ => {
                self.word("import.meta");
                return;
            }
        }
        if self.chance(50) {
            self.function(scope, true);
        } else {
            self.declaration(scope);
        }
    }

    fn head_then_regex(&mut self, scope: Scope) {
        self.pick(&["if (a) ", "while (a) ", "for (;;) ", "a\n++", "with (a) "]);
        self.pick(REGEXES);
        if self.chance(50) {
            self.word(".test(");
            self.operand(scope.inner());
            self.word(")");
        }
    }

    fn jump_then_regex(&mut self, scope: Scope) {
        self.pick(&[
            "for (;;) { break",
            "for (;;) { continue",
            "l: for (;;) { break l",
            "l: while (a) { continue l",
            "as: for (;;) { break as",
            "satisfies: while (a) { continue satisfies",
            "\u{e9}: { break \u{e9}",
            "{ debugger",
        ]);
        self.word("\n");
        self.pick(REGEXES);
        if self.chance(50) {
            self.word(".test(");
            self.operand(scope.inner());
            self.word(")");
        }
        self.word("\n}");
    }

    fn asi_block(&mut self, scope: Scope) {
        self.operand(scope.inner());
        self.word("\n{");
        self.statement(scope.inner());
        self.word("}\n");
        self.pick(REGEXES);
        self.word(".test(b)");
    }

    fn brace_then_regex(&mut self, scope: Scope) {
        match self.below(3) {
            0 => self.word("if (a) {} "),
            1 => self.word("function g() {}\n"),
            _ => self.word("x = {}\n"),
        }
        self.operand(scope.inner());
        self.word(" / ");
        self.operand(scope.inner());
    }

    fn expression(&mut self, scope: Scope) {
        if scope.depth >= MAX_DEPTH {
            self.operand(scope);
            return;
        }
        match self.below(12) {
            0..=3 => self.binary(scope),
            4 => self.conditional(scope),
            5 => self.assignment(scope),
            6 | 7 => self.arrow(scope),
            8 if scope.yields => self.yield_expression(scope),
            9 => {
                self.expression(scope.inner());
                self.word(", ");
                self.expression(scope.inner());
            }
            _ => self.operand(scope),
        }
    }

    fn binary(&mut self, scope: Scope) {
        self.operand(scope.inner());
        for _ in 0..1 + self.below(3) {
            self.gap();
            self.pick(BINARY);
            self.gap();
            self.operand(scope.inner());
        }
    }

    fn conditional(&mut self, scope: Scope) {
        self.operand(scope.inner());
        self.word(" ? ");
        self.expression(scope.inner());
        self.word(" : ");
        self.expression(scope.inner());
    }

    fn assignment(&mut self, scope: Scope) {
        self.pick(&["x", "a.b", "a[0]", "await", "yield"]);
        self.gap();
        self.pick(ASSIGN);
        self.gap();
        self.expression(scope.inner());
    }

    fn arrow(&mut self, scope: Scope) {
        let awaits = self.chance(40);
        if awaits {
            self.pick(&["async ", "async ", "async\n"]);
        }
        self.pick(&["x", "()", "(a, b)", "(a = 1)", "({a})", "([a])"]);
        self.word(" =>");
        self.gap();
        let inner = scope.function(awaits, false);
        if self.chance(40) {
            self.body(inner);
        } else {
            self.expression(inner);
        }
    }

    fn yield_expression(&mut self, scope: Scope) {
        self.pick(&["yield", "yield ", "yield* ", "yield\n"]);
        if self.chance(70) {
            self.operand(scope.inner());
        }
    }

    fn operand(&mut self, scope: Scope) {
        if scope.depth >= MAX_DEPTH {
            self.leaf();
            return;
        }
        match self.below(24) {
            0..=5 => self.leaf(),
            6 => self.template(scope),
            7 => self.parenthesized(scope),
            8 => self.array(scope),
            9 => self.object(scope),
            10 => self.function(scope, false),
            11 => {
                if self.chance(30) {
                    self.pick(&["@a ", "@a.b ", "@(a) ", "@a() "]);
                }
                self.class(scope);
            }
            12 | 13 => self.access(scope),
            14 => self.unary(scope),
            15 => self.update(),
            16 | 17 if scope.awaits => self.await_expression(scope),
            18 => self.jsx(scope),
            19 => self.arrow(scope),
            20 => self.new_expression(scope),
            _ => self.leaf(),
        }
    }

    fn leaf(&mut self) {
        match self.below(6) {
            0 | 1 => self.pick(NAMES),
            2 => self.pick(NUMBERS),
            3 => self.pick(STRINGS),
            4 => self.pick(REGEXES),
            _ => self.pick(&["this", "null", "true", "x", "`plain / ( `"]),
        }
    }

    fn template(&mut self, scope: Scope) {
        self.word("`a /* ( ");
        for _ in 0..self.below(3) {
            self.word("${");
            self.expression(scope.inner());
            self.word("} ' [");
        }
        self.word("`");
    }

    fn parenthesized(&mut self, scope: Scope) {
        self.word("(");
        self.expression(scope.inner());
        self.word(")");
        if self.chance(30) {
            self.gap();
            self.word("/ 2");
        }
    }

    fn array(&mut self, scope: Scope) {
        self.word("[");
        self.expression(scope.inner());
        self.pick(&[", ", ", , ", ", ..."]);
        self.operand(scope.inner());
        self.word("]");
    }

    fn object(&mut self, scope: Scope) {
        let bare = self.chance(40);
        self.word(if bare { "{" } else { "({" });
        for index in 0..1 + self.below(3) {
            if index > 0 {
                self.word(",");
                self.gap();
            }
            self.object_member(scope.inner());
        }
        self.word(if bare { "}" } else { "})" });
    }

    fn object_member(&mut self, scope: Scope) {
        match self.below(4) {
            0 => self.method(scope),
            1 => {
                self.pick(&["b", "async", "get", "await", "yield", "of"]);
            }
            2 => {
                self.word("...");
                self.operand(scope);
            }
            _ => {
                self.pick(&["a", "if", "class", "'s'", "1", "[k]", "await", "get"]);
                self.word(": ");
                self.expression(scope);
            }
        }
    }

    fn access(&mut self, scope: Scope) {
        self.leaf();
        match self.below(4) {
            0 => match self.below(5) {
                0 => {
                    self.word("?.(");
                    self.leaf();
                    self.word(")");
                }
                1 => {
                    self.word("?.[");
                    self.leaf();
                    self.word("]");
                }
                _ => {
                    self.pick(&[".", "?.", "\n."]);
                    self.pick(MEMBERS);
                }
            },
            1 => {
                self.word("[");
                self.expression(scope.inner());
                self.word("]");
            }
            2 => {
                self.word("(");
                self.expression(scope.inner());
                self.word(")");
            }
            _ => {
                self.word("`t${");
                self.expression(scope.inner());
                self.word("}`");
            }
        }
    }

    fn unary(&mut self, scope: Scope) {
        self.pick(&["!", "-", "~", "typeof ", "void ", "delete ", "+"]);
        self.operand(scope.inner());
    }

    fn update(&mut self) {
        if self.chance(50) {
            self.pick(&["++", "--"]);
            self.pick(&["a", "a.b"]);
        } else {
            self.pick(&["a", "a.b"]);
            self.pick(&["++", "--"]);
        }
        if self.chance(40) {
            self.pick(SEPARATORS);
            self.pick(&["/ 2", "/ b / g", "++c"]);
        }
    }

    fn await_expression(&mut self, scope: Scope) {
        self.word("await");
        self.pick(&[" ", " ", "\n", "/**/"]);
        match self.below(5) {
            0 => self.pick(REGEXES),
            1 => self.function(scope.inner(), false),
            2 => self.jsx(scope.inner()),
            _ => self.operand(scope.inner()),
        }
    }

    fn jsx(&mut self, scope: Scope) {
        let fragment = self.chance(20);
        self.word(if fragment { "<>" } else { "<a" });
        if !fragment {
            self.jsx_attributes(scope);
            if self.chance(25) {
                self.word(" />");
                return;
            }
            self.word(">");
        }
        for _ in 0..self.below(3) {
            if self.chance(50) {
                self.pick(JSX_TEXT);
            } else {
                self.word("{");
                self.expression(scope.inner());
                self.word("}");
            }
        }
        if self.chance(30) {
            self.word("<b />");
        }
        self.word(if fragment { "</>" } else { "</a>" });
    }

    fn jsx_attributes(&mut self, scope: Scope) {
        for _ in 0..self.below(3) {
            self.gap();
            match self.below(3) {
                0 => self.word("b=\"x'\""),
                1 => self.word("c='\"/('"),
                _ => {
                    self.word("d={");
                    self.expression(scope.inner());
                    self.word("}");
                }
            }
        }
    }

    fn new_expression(&mut self, scope: Scope) {
        self.word("new ");
        self.pick(&["a", "a.b", "A"]);
        if self.chance(50) {
            self.word("(");
            self.expression(scope.inner());
            self.word(")");
        }
    }
}
