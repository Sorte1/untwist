use std::path::Path;

use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, patched_after_rotator, Mode};

const GLOBAL: &str = "Function('return this')()";
const STRING_REPLACED: &str = "{ fromCharCode: function () { return '?'; } }";
const PATCH: &str = "function () { return '?'; }";
const STRING_TEST: &str = "String.fromCharCode(33) === '!'";
const JOIN_TEST: &str = "[1, 2].join('') === '12'";

fn mismatches(node: &Path, programs: &[(String, String)]) -> Vec<String> {
    let conservative = Options {
        extended: false,
        unsafe_transforms: true,
        ..Options::default()
    };
    let run_default = |source: &str| {
        Pipeline::new(Options::default())
            .run(source)
            .code()
            .to_string()
    };
    let run_conservative = |source: &str| {
        Pipeline::new(conservative.clone())
            .run(source)
            .code()
            .to_string()
    };
    let runs: [Mode<'_>; 2] = [
        ("default", &run_default),
        ("conservative", &run_conservative),
    ];
    node_rows::mismatches(node, programs, &runs)
}

fn assert_all_print_like_node(rows: Vec<(&str, String)>) {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = rows
        .into_iter()
        .map(|(label, source)| (label.to_string(), source))
        .collect();
    let wrong = mismatches(&node, &programs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

fn after_rotator(rows: Vec<(&'static str, &str, String)>) -> Vec<(&'static str, String)> {
    rows.into_iter()
        .map(|(label, test, patch)| (label, patched_after_rotator(test, &patch)))
        .collect()
}

fn definer_spellings() -> Vec<(&'static str, &'static str, String)> {
    let (g, r, t) = (GLOBAL, STRING_REPLACED, STRING_TEST);
    vec![
        ("defprop-call", t, format!("Object.defineProperty.call(null, {g}, 'String', {{ value: {r} }});")),
        ("defprop-apply", t, format!("Reflect.apply(Object.defineProperty, null, [{g}, 'String', {{ value: {r} }}]);")),
        ("defprop-alias", t, format!("var dp = Object.defineProperty; dp({g}, 'String', {{ value: {r} }});")),
        ("defprop-destructured", t, format!("var {{ defineProperty: dp }} = Object; dp({g}, 'String', {{ value: {r} }});")),
        ("defprop-bind", t, format!("Object.defineProperty.bind(null, {g})('String', {{ value: {r} }});")),
        ("define-getter-call", t, format!("Object.prototype.__defineGetter__.call({g}, 'String', function () {{ return {r}; }});")),
        ("assign-variable", t, format!("var src = {{ String: {r} }}; Object.assign({g}, src);")),
        ("assign-spread", t, format!("Object.assign({g}, {{ ...{{ String: {r} }} }});")),
        ("define-properties-variable", t, format!("var d = {{ String: {{ value: {r} }} }}; Object.defineProperties({g}, d);")),
        ("join-defprop-alias", JOIN_TEST, format!("var dp = Object.defineProperty; dp(Array.prototype, 'join', {{ value: {PATCH} }});")),
        ("join-defprop-call", JOIN_TEST, format!("Object.defineProperty.call(null, Array.prototype, 'join', {{ value: {PATCH} }});")),
        ("join-define-getter", JOIN_TEST, format!("Array.prototype.__defineGetter__('join', function () {{ return {PATCH}; }});")),
    ]
}

#[test]
fn a_definer_called_any_other_way_or_given_unlisted_keys_prints_like_node() {
    assert_all_print_like_node(after_rotator(definer_spellings()));
}

#[test]
fn a_literal_chain_beside_a_definer_called_any_other_way_prints_like_node() {
    let tail = "console.log([1, 2].join(''));";
    assert_all_print_like_node(vec![
        ("fold-defprop-call", format!("Object.defineProperty.call(null, Array.prototype, 'join', {{ value: {PATCH} }}); {tail}")),
        ("fold-assign-bind", format!("Object.assign.bind(null, Array.prototype)({{ join: {PATCH} }}); {tail}")),
        ("fold-define-getter-call", format!("Object.prototype.__defineGetter__.call(Array.prototype, 'join', function () {{ return {PATCH}; }}); {tail}")),
    ]);
}

fn halves_after_rotator(patch: &str) -> String {
    format!(
        "function a() {{ var h = ['Hello World!', 'console', 'deobfuscate', 'log', 'Str', 'ing', 'fromChar', 'Code']; a = function () {{ return h; }}; return a(); }}
function b(c, d) {{
var e = a();
return (b = function (f, g) {{
f = f - 0;
var i = e[f];
return i + ({STRING_TEST} ? '' : '?');
}}), b(c, d);
}}
(function (c, d) {{
var e = c();
var tries = 0;
while (!![]) {{
try {{
if (b(0) === 'log') break;
else e['push'](e['shift']());
}} catch (h) {{ e['push'](e['shift']()); }}
if (tries++ > 1000) break;
}}
}})(a, 0);
{patch}
console.log(b(0));
console.log(b(1));
console.log(b(3));
"
    )
}

#[test]
fn a_key_joined_from_decoded_halves_any_static_way_prints_like_node() {
    let (g, r) = (GLOBAL, STRING_REPLACED);
    let rows = [
        ("template", format!("{g}[`${{b(1)}}${{b(2)}}`] = {r};")),
        (
            "define-property-template",
            format!("Object.defineProperty({g}, `${{b(1)}}${{b(2)}}`, {{ value: {r} }});"),
        ),
        (
            "assign-template",
            format!("Object.assign({g}, {{ [`${{b(1)}}${{b(2)}}`]: {r} }});"),
        ),
        ("array-join", format!("{g}[[b(1), b(2)].join('')] = {r};")),
        ("concat-call", format!("{g}[b(1).concat(b(2))] = {r};")),
        (
            "template-from-char-code",
            format!("''.constructor[`${{b(3)}}${{b(4)}}`] = {PATCH};"),
        ),
    ];
    let rows = rows
        .into_iter()
        .map(|(label, patch)| (label, halves_after_rotator(&patch)))
        .collect();
    assert_all_print_like_node(rows);
}

#[test]
fn a_key_spelled_statically_any_resolvable_way_prints_like_node() {
    let (g, r, t) = (GLOBAL, STRING_REPLACED, STRING_TEST);
    assert_all_print_like_node(after_rotator(vec![
        (
            "one-element-array-key",
            t,
            format!("{g}[['String']] = {r};"),
        ),
        (
            "literal-concat-key",
            t,
            format!("{g}['Str'.concat('ing')] = {r};"),
        ),
        (
            "literal-template-key",
            t,
            format!("{g}[`${{'Str'}}ing`] = {r};"),
        ),
        (
            "literal-join-key",
            t,
            format!("{g}[['Str', 'ing'].join('')] = {r};"),
        ),
    ]));
}

const LOOKUP_TEST: &str = "typeof b.hasOwnProperty === 'function'";
const CHAR_AT_TEST: &str = "'abc'.charAt(0) === 'a'";

const GLOBAL_PROTOTYPE_WRITES: &[(&str, &str, &str)] = &[
    (
        "global-this-cut",
        LOOKUP_TEST,
        "globalThis.Function.prototype.__proto__ = null;",
    ),
    (
        "global-cut",
        LOOKUP_TEST,
        "global.Function.prototype.__proto__ = null;",
    ),
    (
        "global-this-computed-cut",
        LOOKUP_TEST,
        "globalThis['Function'].prototype.__proto__ = null;",
    ),
    (
        "global-this-alias-cut",
        LOOKUP_TEST,
        "var g = globalThis; g.Function.prototype.__proto__ = null;",
    ),
    (
        "prototype-alias-through-global-this",
        LOOKUP_TEST,
        "var FP = globalThis.Function.prototype; FP.__proto__ = null;",
    ),
    (
        "global-this-replaced-lookup",
        LOOKUP_TEST,
        "globalThis.Object.prototype.hasOwnProperty = 1;",
    ),
    (
        "global-this-join",
        JOIN_TEST,
        "globalThis.Array.prototype.join = function () { return '?'; };",
    ),
    (
        "global-join",
        JOIN_TEST,
        "global.Array.prototype.join = function () { return '?'; };",
    ),
    (
        "global-this-char-at",
        CHAR_AT_TEST,
        "globalThis.String.prototype.charAt = function () { return '?'; };",
    ),
    (
        "global-this-alias-char-code",
        "'-'.charCodeAt(0) === 45",
        "var g = globalThis; g.String.prototype.charCodeAt = function () { return 1; };",
    ),
];

#[test]
fn a_prototype_written_through_a_spelled_global_object_prints_like_node() {
    let rows = GLOBAL_PROTOTYPE_WRITES
        .iter()
        .map(|(label, test, patch)| (*label, *test, patch.to_string()))
        .collect();
    assert_all_print_like_node(after_rotator(rows));
}

#[test]
fn a_literal_chain_beside_a_prototype_written_through_a_global_object_prints_like_node() {
    assert_all_print_like_node(vec![
        ("fold-global-this-join", format!("globalThis.Array.prototype.join = {PATCH}; console.log([1, 2].join(''));")),
        ("fold-global-this-alias", format!("var g = globalThis; g.String.prototype.charAt = {PATCH}; console.log('abc'.charAt(0));")),
        ("fold-window-alias-later", format!("function f() {{ w.Array.prototype.join = {PATCH}; }} var w = global; f(); console.log([1, 2].join(''));")),
    ]);
}

#[test]
fn a_prototype_written_through_a_constructor_alias_or_with_prints_like_node() {
    let rows = vec![
        (
            "constructor-alias-join",
            JOIN_TEST,
            format!("var A = Array; A.prototype.join = {PATCH};"),
        ),
        (
            "string-constructor-alias",
            CHAR_AT_TEST,
            format!("var S = ''.constructor; S.prototype.charAt = {PATCH};"),
        ),
        (
            "array-constructor-alias",
            JOIN_TEST,
            format!("var A = [].constructor; A.prototype.join = {PATCH};"),
        ),
    ];
    assert_all_print_like_node(after_rotator(rows));
}

#[test]
fn a_literal_chain_beside_a_constructor_alias_or_with_prints_like_node() {
    assert_all_print_like_node(vec![
        ("fold-constructor-alias", format!("var A = Array; A.prototype.join = {PATCH}; console.log([1, 2].join(''));")),
        ("fold-alias-in-function", format!("function f() {{ var S = ''.constructor; S.prototype.charAt = {PATCH}; }} f(); console.log('abc'.charAt(0));")),
        ("fold-alias-declared-later", format!("function f() {{ A.prototype.join = {PATCH}; }} var A = Array; f(); console.log([1, 2].join(''));")),
        ("fold-with", format!("with (Array.prototype) {{ join = {PATCH}; }} console.log([1, 2].join(''));")),
    ]);
}

const SPECIES: &str = "class X extends Array { join() { return '?'; } }";

#[test]
fn an_array_created_through_a_replaced_species_or_constructor_prints_like_node() {
    let by_species =
        format!("{SPECIES} Object.defineProperty(Array, Symbol.species, {{ value: X }});");
    let by_constructor = format!("{SPECIES} Array.prototype.constructor = X;");
    let rows = vec![
        ("species-concat", "[1].concat([2]).join('') === '12'", by_species.clone()),
        ("species-slice", "[1, 2].slice(0).join('') === '12'", by_species.clone()),
        ("species-filter", "[1, 2].filter(function () { return true; }).join('') === '12'", by_species.clone()),
        ("species-computed", "[1, 2].slice(0).join('') === '12'", format!("{SPECIES} var k = Symbol.species; Object.defineProperty(Array, k, {{ value: X }});")),
        ("constructor-slice", "[1, 2].slice(0).join('') === '12'", by_constructor.clone()),
        ("constructor-splice", "[1, 2].splice(0).join('') === '12'", by_constructor),
    ];
    assert_all_print_like_node(after_rotator(rows));
}

#[test]
fn a_literal_chain_beside_a_replaced_species_prints_like_node() {
    assert_all_print_like_node(vec![(
        "fold-species",
        format!("{SPECIES} Object.defineProperty(Array, Symbol.species, {{ value: X }}); console.log([1].concat([2]).join(''), [1, 2].slice(0).join(''));"),
    )]);
}
