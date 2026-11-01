use oxc_allocator::Allocator;
use oxc_ast::ast::{JSXElement, JSXFragment, RegExpLiteral, TemplateLiteral};
use oxc_ast_visit::{walk, Visit};
use untwist_core::depth::{scan, scan_as, Goal, UNLIMITED};
use untwist_core::parse::{has_error, parse_with, SourceType};

#[derive(Default, Debug, PartialEq)]
struct Literals {
    regexes: u32,
    templates: u32,
    jsx: u32,
}

impl<'a> Visit<'a> for Literals {
    fn visit_reg_exp_literal(&mut self, _it: &RegExpLiteral<'a>) {
        self.regexes += 1;
    }

    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        self.templates += 1;
        walk::walk_template_literal(self, it);
    }

    fn visit_jsx_element(&mut self, it: &JSXElement<'a>) {
        self.jsx += 1;
        walk::walk_jsx_element(self, it);
    }

    fn visit_jsx_fragment(&mut self, it: &JSXFragment<'a>) {
        self.jsx += 1;
        walk::walk_jsx_fragment(self, it);
    }
}

const M: Goal = Goal::Module;
const S: Goal = Goal::Script;

fn parsed(source: &str, goal: Goal) -> Literals {
    let alloc = Allocator::default();
    let source_type = match goal {
        Goal::Module => SourceType::mjs(),
        Goal::Script => SourceType::cjs(),
    };
    let result = parse_with(&alloc, source, source_type.with_jsx(true));
    assert!(
        !has_error(&result.diagnostics),
        "{goal:?} does not parse {source:?}"
    );
    let mut literals = Literals::default();
    literals.visit_program(&result.program);
    literals
}

fn scanned(source: &str, goal: Goal) -> Literals {
    let result = scan_as(source, &UNLIMITED, goal).expect("an unlimited scan admits every input");
    Literals {
        regexes: result.regexes,
        templates: result.templates,
        jsx: result.jsx_elements,
    }
}

fn agree(rows: &[(&str, Goal)]) {
    for &(source, goal) in rows {
        assert_eq!(
            scanned(source, goal),
            parsed(source, goal),
            "{goal:?}: {source:?}"
        );
        assert!(scan(source, &UNLIMITED).is_ok());
    }
}

#[test]
fn await_reads_a_regex_or_jsx_where_it_is_a_keyword() {
    agree(&[
        ("await/)/", M),
        ("await<a></a>", M),
        ("async function f(){await/'/}", S),
        ("async()=>await<a></a>", S),
        ("function f(){x = await / 2 / 1}", S),
        ("function f(){x = await / 2 / 1}", M),
        ("async function f(){y=>3\nfor(x of await<a></a>)b}", S),
    ]);
}

#[test]
fn a_brace_after_a_name_and_a_newline_is_a_block() {
    agree(&[
        ("a\n{}/)/", M),
        ("x\n{}/'/()", S),
        ("try{u\n{}/'/()}finally{}", S),
        ("var t\n<a></a>", S),
    ]);
}

#[test]
fn a_brace_after_a_closing_brace_is_a_block() {
    agree(&[
        ("{}{}/\"/", S),
        ("try{}finally{}{}/\\//", S),
        ("do{{}/x/()}while(b)", S),
        ("for(function(){try{<a>{}</a>}finally{}};{s(){}}<x;)(y)", S),
    ]);
}

#[test]
fn a_conditional_colon_continues_the_expression() {
    agree(&[
        ("t?s:{}<(d)", S),
        ("x?b:class{}<f;{if((async function(){})){}}try{}catch{}", S),
        ("(5)?'':function(){}<a", S),
        (
            "class extends[a?l:'']{static async*m(){({})/await<a></a>}}",
            S,
        ),
    ]);
}

#[test]
fn an_async_function_takes_the_token_before_async() {
    agree(&[
        ("/**/void async function(){}<[]", S),
        ("for(x of async function(){}<c);", S),
        ("async x=>await\nasync function(){}/``", S),
        ("(F^await async function(){}<o)", M),
    ]);
}

#[test]
fn of_after_a_newline_and_function_after_of() {
    agree(&[("x\nof/<a/>", S), ("for(x of function(){}<c)/(/", S)]);
}

#[test]
fn a_binding_name_ends_its_declaration_at_a_newline() {
    agree(&[
        ("var{}=(n),t\n/\"/>async function(){}", S),
        ("var{}=h?a:5,t\n<a></a>", S),
        ("var{}=h?a:5,await\n<a></a>", S),
        ("let x\n/=x/.test(b)", M),
    ]);
}

#[test]
fn let_is_a_name_outside_a_statement_list() {
    agree(&[
        ("typeof let\n{}/x/", S),
        ("{while({})let\n{}/\"/()}", S),
        ("f:let\n{([])>{}}/)/()", S),
        ("u|let\n{}/x/", S),
        ("for(x in m)try{}finally{}let.f,async()=>await<a></a>", M),
        ("await\nlet\n{}/ /", M),
    ]);
}

#[test]
fn static_blocks_and_class_heritage() {
    agree(&[
        ("try{}catch{class{1(){}static{{}/)/()}}}", S),
        ("class extends[{}]{static async*m(){({})/await<a></a>}}", S),
        ("class extends/x/[a=>{{}/\\//}]{}2?([]):1", S),
    ]);
}

#[test]
fn methods_carry_their_own_await_and_yield() {
    agree(&[
        ("class{async[k](){if((b))await/'/}}", S),
        ("class{async class(){f(await<></>)}}", M),
        ("({*g(){yield/x/}, m(){yield/2/1}})", S),
        ("class{x=await/2/1; async m(){await/x/}}", S),
        ("class{''(){}[k]=class{}<r}", S),
    ]);
}

#[test]
fn arrow_bodies_end_where_the_parser_ends_them() {
    agree(&[
        ("[x=>/`/]([]=await/`/)*\"\"", M),
        ("p?()=>()=>s:/c/<await/\\//", M),
        ("()=>()=>{(c)+[]}\n[/x/?l:await<a b=\"\"><b/></a>]", M),
        ("()=>t\n!await/\\//", M),
        ("()=>``^<a/>\n[]<await/\"/", M),
    ]);
}

#[test]
fn statement_heads_after_for_await_export_and_update() {
    agree(&[
        ("async x=>{await<a b=\"\"></a>;for await(x of/x/)/\\//}", S),
        ("export default function(){}/)/", M),
        ("export default async function(){}/)/;try{}finally{}", S),
        ("a\n++/)/[b]\n!``", M),
    ]);
}

#[test]
fn html_comments_follow_the_goal() {
    agree(&[
        ("\n<!-- (( ` text\n a <!-- ` ((\n ", S),
        ("x\n--> `\n", S),
        ("a <!-- b", M),
        ("a <!-- b", S),
    ]);
}

#[test]
fn a_newline_ends_break_continue_debugger_and_module_declarations() {
    agree(&[
        ("for(;;){ break\n/`/;\nx\n;/`/ }", S),
        ("for(;;){ continue\n/`/;\nx\n;/`/ }", S),
        ("debugger\n/`/;\nx\n;/`/", S),
        ("l: for(;;) break l\n/`/;\nx\n;/`/", S),
        ("l: for(;;) { continue l\n/x/g }", S),
        ("import a from 'b'\n/`/;\nx\n;/`/", M),
        ("import 'b'\n/`/;\nx\n;/`/", M),
        ("export * from 'y'\n/`/;\nx\n;/`/", M),
        ("export {a as b} from 'y'\n<a></a>", M),
        ("import a from 'b' with {type: 'json'}\n/`/", M),
        ("from\n'y'\n/x/g", S),
        ("let from = 1\nfrom\n/x/g", S),
    ]);
}

#[test]
fn of_then_a_newline_then_a_brace_opens_an_object() {
    agree(&[
        ("for (x of\n{} /x/ 1);", S),
        ("for (x of\n{}/`/`);", S),
        ("of\n{}/x/", S),
    ]);
}

#[test]
fn import_attributes_spelled_assert_end_the_declaration() {
    agree(&[
        ("import a from 'a' assert {type: 'json'}\n/`/;\nx\n;/`/", M),
        ("import 'a' assert {}\n/`/;\nx\n;/`/", M),
        ("export * from 'a' assert {}\n/`/;\nx\n;/`/", M),
        ("export {a} from 'a' assert {}\n/x/g", M),
        ("import a from 'a'\nwith {type: 'json'}\n/x/g", M),
        ("import a from 'a'\nassert\n{}/x/", M),
    ]);
}

#[test]
fn a_decorated_class_expression_is_an_operand() {
    agree(&[
        ("x / @a class{} / 2 / 1", S),
        ("x = @a class{} / 2 / 1", S),
        ("(@a class{} / 2 / 1)", S),
        ("new @a class{} / 2", S),
        ("async function f(){ await @a class{} / 2 }", S),
        ("@a class B {}\n/x/g", S),
        ("@a.b(c) @d class B {}\n/x/g", S),
    ]);
}

#[test]
fn the_second_of_after_a_declared_of_is_the_loop_keyword() {
    agree(&[
        ("for (let of of class{} / 2 /1);", S),
        ("for (var of of class{} / 2 /1);", S),
        ("for (const of of [] / 2 /1);", S),
    ]);
}

#[test]
fn javascript_that_spells_typescript_words_is_scanned() {
    agree(&[
        ("import {a as b} from 'c'\n/x/g", M),
        ("import * as a from 'c'\n/x/g", M),
        ("import {'a' as b, default as c} from 'd'\n/x/g", M),
        ("export {a as b, c as default}\n/x/g", M),
        ("export * as a from 'b'\n/x/g", M),
        ("let as = 1, implements = 2\nas / 2 / 1", S),
        ("var as; function as(){} as / 2 / 1", S),
        ("({get as(){}, set as(v){}, as(){}, async as(){}})", S),
        (
            "class A { foo(){} as(){} static as(){} get as(){} static async as(){} }",
            S,
        ),
        (
            "class A { static x = [a, b]; [k] = 1; [Symbol.iterator](){} [a ? b : c](){} }",
            S,
        ),
        ("class A extends f(a < b) {}\n/x/g", S),
        ("async as => as / 2 / 1", S),
        ("x.as / 2, a?.satisfies / 2", S),
        ("async function f(){ await as / 2 }", S),
        ("async as => 1", S),
        ("async as /*c*/ => 1", S),
        ("for (let as = 0; as < 2; as++) as / 2 / 1;", S),
        ("for (let as of x) as / 2 / 1;", M),
        ("for (let as in x) as / 2 / 1;", S),
        ("for (let as; ;) break;", M),
        ("for await (let as of x) as / 2 / 1;", M),
    ]);
    let both: Vec<(&str, Goal)> = SPELLED_AS_NAMES
        .iter()
        .flat_map(|&source| [(source, M), (source, S)])
        .collect();
    agree(&both);
}

const SPELLED_AS_NAMES: &[&str] = &[
    "as: for(;;) break as\n/x/g",
    "as: for(;;) continue as\n/x/g",
    "as: { break as }\n/x/g",
    "satisfies: for(;;) break satisfies\n/x/g",
    "class A { @dec as(){} }\n/x/g",
    "class A { @dec() as = 1 }\n/x/g",
    "class A { @a.b as(){} }\n/x/g",
    "class A { @dec get as(){} }\n/x/g",
    "class A { @dec static async as(){} }\n/x/g",
    "class A { @dec x(){} @dec as(){} }\n/x/g",
    "class A { @dec satisfies(){} }\n/x/g",
    "x = class { @dec as(){} }\n/x/g",
    "class A { [\u{e9}](){} }\n/x/g",
    "class A { [\u{e9}a](){} }\n/x/g",
    "class A { [\u{fc}] = 1 }\n/x/g",
    "class A { static [\u{e9}](){} }\n/x/g",
    "class A { [ /*x*/ \u{e9} ](){} }\n/x/g",
    "class A extends f(class { [\u{e9}](){} }) {}\n/x/g",
];

#[test]
fn a_single_statement_let_declares_only_before_a_bracket() {
    agree(&[
        ("if (a) let\n{}\n/x/g", S),
        ("l: let\n{}\n/x/g", S),
        ("while (a) let\nx\n/x/g", S),
    ]);
}

#[test]
fn member_names_the_oracles_found_are_scanned() {
    let both: Vec<(&str, Goal)> = [
        "class A{*class(){}as=1}\n/x/g",
        "class A extends[()=>{class B{static as(){}}}]{}\n/x/g",
        "class A{#p\n@d as(){}}\n/x/g",
        "class A{e\n@t as(){}}\n/x/g",
        "class A{@[]satisfies=1}\n/x/g",
        "class A{[k]\n@t satisfies(){}}\n/x/g",
        "class A{class(){} @dec as = 1}\n/x/g",
        "class A{get class(){} @dec as = 1}\n/x/g",
        "class A{static function\n@dec satisfies(){}}\n/x/g",
        "class A{[s]\nclass = 1; @d as(){}}\n/x/g",
        "class A{x = @a() class{static{} @c as(){}}\nc(){}}\n/x/g",
        "class A{e = @b class{@r as = 1}}\n/x/g",
    ]
    .iter()
    .flat_map(|&source| [(source, M), (source, S)])
    .collect();
    agree(&both);
    agree(&[
        (
            "export * as function from 'm';\nx = { set as(v){} }\n/x/g",
            M,
        ),
        ("import {if as y} from 'm'\n/x/g", M),
        ("import {as as as} from 'm'\n/x/g", M),
        ("import as from 'm'\n/x/g", M),
        ("export {x as default, y as if} from 'm'\n/x/g", M),
    ]);
}
