use std::collections::HashMap as FxHashMap;
use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_span::GetSpan;
use oxc_syntax::scope::ScopeFlags;
use untwist::{Options, Pipeline, RenameMode};
use untwist_core::ast_util::naming_slot::reads_names;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_core::source_map::OwnedSourceMap;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");
const NAME_READ: &str =
    "\n;globalThis.__untwist_name = (globalThis.__untwist_probe || Object).name;\n";
const EVERY_FILE: [&str; 13] = [
    "behaviour",
    "dead-code",
    "unminify",
    "spelled-name",
    "literal-branches",
    "pure-calls",
    "obfuscated",
    "layers",
    "eval",
    "jsx",
    "deobfuscate-gaps",
    "self-defending",
    "builtin-replaced",
];
const SAMPLED: [(&str, usize); 2] = [("untranspile", 16), ("unpack-bundlers", 1)];
const UNMAPPED_CEILING: f64 = 0.75;

#[derive(Clone, Debug, PartialEq)]
enum Inferred {
    Name(String),
    Unknown,
}

struct Names<'t> {
    text: &'t str,
    found: FxHashMap<u32, Inferred>,
}

fn strip<'e, 'a>(expr: &'e Expression<'a>) -> &'e Expression<'a> {
    expr.without_parentheses()
}

fn key_name(key: &PropertyKey<'_>, computed: bool) -> Inferred {
    match key {
        PropertyKey::StaticIdentifier(id) if !computed => Inferred::Name(id.name.to_string()),
        PropertyKey::PrivateIdentifier(id) => Inferred::Name(format!("#{}", id.name)),
        PropertyKey::StringLiteral(literal) => literal
            .value
            .as_str()
            .map_or(Inferred::Unknown, |value| Inferred::Name(value.to_string())),
        PropertyKey::NumericLiteral(literal) if !computed => {
            Inferred::Name(literal.value.to_string())
        }
        _ => Inferred::Unknown,
    }
}

fn prefixed(kind: PropertyKind, name: Inferred) -> Inferred {
    match (kind, name) {
        (PropertyKind::Get, Inferred::Name(name)) => Inferred::Name(format!("get {name}")),
        (PropertyKind::Set, Inferred::Name(name)) => Inferred::Name(format!("set {name}")),
        (_, name) => name,
    }
}

fn method_prefixed(kind: MethodDefinitionKind, name: Inferred) -> Inferred {
    match kind {
        MethodDefinitionKind::Get => prefixed(PropertyKind::Get, name),
        MethodDefinitionKind::Set => prefixed(PropertyKind::Set, name),
        _ => name,
    }
}

impl Names<'_> {
    fn name_value(&mut self, value: &Expression<'_>, name: Inferred) {
        let value = strip(value);
        if value.is_anonymous_function_definition() {
            self.found.entry(value.span().start).or_insert(name);
        }
    }

    fn name_binding(&mut self, pattern: &BindingPattern<'_>, value: &Expression<'_>) {
        if let BindingPattern::BindingIdentifier(id) = pattern {
            self.name_value(value, Inferred::Name(id.name.to_string()));
        }
    }

    fn target_parenthesised(&self, outer: u32, id: &IdentifierReference<'_>) -> bool {
        self.text
            .get(outer as usize..id.span.start as usize)
            .is_some_and(|between| between.contains('('))
    }

    fn record(&mut self, start: u32, name: Inferred) {
        self.found.entry(start).or_insert(name);
    }
}

impl<'a> Visit<'a> for Names<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let name = it.id.as_ref().map_or(Inferred::Name(String::new()), |id| {
            Inferred::Name(id.name.to_string())
        });
        if it.id.is_some() {
            self.found.insert(it.span.start, name);
        } else {
            self.record(it.span.start, name);
        }
        walk::walk_function(self, it, flags);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        let name = it.id.as_ref().map_or(Inferred::Name(String::new()), |id| {
            Inferred::Name(id.name.to_string())
        });
        let start = it
            .decorators
            .first()
            .map_or(it.span.start, |decorator| decorator.span.start);
        if it.id.is_some() {
            self.found.insert(start, name);
        } else {
            self.record(start, name);
        }
        walk::walk_class(self, it);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.record(it.span.start, Inferred::Name(String::new()));
        walk::walk_arrow_function_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let Some(init) = &it.init {
            self.name_binding(&it.id, init);
        }
        walk::walk_variable_declarator(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        let naming = matches!(
            it.operator,
            AssignmentOperator::Assign
                | AssignmentOperator::LogicalOr
                | AssignmentOperator::LogicalAnd
                | AssignmentOperator::LogicalNullish
        );
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left {
            if naming {
                let name = match self.target_parenthesised(it.span.start, id) {
                    true => Inferred::Name(String::new()),
                    false => Inferred::Name(id.name.to_string()),
                };
                self.name_value(&it.right, name);
            }
        }
        walk::walk_assignment_expression(self, it);
    }

    fn visit_assignment_pattern(&mut self, it: &AssignmentPattern<'a>) {
        self.name_binding(&it.left, &it.right);
        walk::walk_assignment_pattern(self, it);
    }

    fn visit_formal_parameter(&mut self, it: &FormalParameter<'a>) {
        if let Some(init) = &it.initializer {
            self.name_binding(&it.pattern, init);
        }
        walk::walk_formal_parameter(self, it);
    }

    fn visit_assignment_target_with_default(&mut self, it: &AssignmentTargetWithDefault<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.binding {
            self.name_value(&it.init, Inferred::Name(id.name.to_string()));
        }
        walk::walk_assignment_target_with_default(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        if let Some(init) = &it.init {
            self.name_value(init, Inferred::Name(it.binding.name.to_string()));
        }
        walk::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        let proto = !it.computed
            && matches!(&it.key, PropertyKey::StaticIdentifier(id) if id.name == "__proto__")
            || matches!(&it.key, PropertyKey::StringLiteral(s) if !it.computed && s.value == "__proto__");
        if !(proto && it.kind == PropertyKind::Init && !it.method && !it.shorthand) {
            let name = prefixed(it.kind, key_name(&it.key, it.computed));
            self.name_value(&it.value, name);
        }
        walk::walk_object_property(self, it);
    }

    fn visit_method_definition(&mut self, it: &MethodDefinition<'a>) {
        let name = method_prefixed(it.kind, key_name(&it.key, it.computed));
        if it.value.id.is_none() {
            self.record(it.value.span.start, name);
        }
        walk::walk_method_definition(self, it);
    }

    fn visit_property_definition(&mut self, it: &PropertyDefinition<'a>) {
        if let Some(value) = &it.value {
            self.name_value(value, key_name(&it.key, it.computed));
        }
        walk::walk_property_definition(self, it);
    }

    fn visit_export_default_declaration(&mut self, it: &ExportDefaultDeclaration<'a>) {
        let start = it.declaration.span().start;
        let anonymous = match &it.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(function) => function.id.is_none(),
            ExportDefaultDeclarationKind::ClassDeclaration(class) => class.id.is_none(),
            other => other
                .as_expression()
                .is_some_and(|expr| strip(expr).is_anonymous_function_definition()),
        };
        if anonymous {
            let start = match it.declaration.as_expression() {
                Some(expr) => strip(expr).span().start,
                None => start,
            };
            self.record(start, Inferred::Name("default".to_string()));
        }
        walk::walk_export_default_declaration(self, it);
    }
}

fn inferred_names(program: &Program<'_>) -> FxHashMap<u32, Inferred> {
    let mut names = Names {
        text: program.source_text,
        found: FxHashMap::default(),
    };
    names.visit_program(program);
    names.found
}

fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(at, _)| at + 1))
        .collect()
}

fn position(text: &str, starts: &[usize], offset: usize) -> (u32, u32) {
    let line = starts.partition_point(|start| *start <= offset) - 1;
    let column: usize = text[starts[line]..offset]
        .chars()
        .map(char::len_utf16)
        .sum();
    (line as u32, column as u32)
}

fn offset(text: &str, starts: &[usize], (line, column): (u32, u32)) -> Option<u32> {
    let start = *starts.get(line as usize)?;
    let mut units = 0u32;
    for (at, ch) in text[start..].char_indices() {
        if units == column {
            return Some((start + at) as u32);
        }
        units += ch.len_utf16() as u32;
    }
    (units == column).then_some(text.len() as u32)
}

#[derive(Default)]
struct Tally {
    compared: usize,
    unmapped: usize,
    untokened: usize,
    trees: usize,
    unmapped_modules: usize,
}

fn differences(
    label: &str,
    (input, output): (&str, &str),
    map: &OwnedSourceMap,
    tally: &mut Tally,
) -> Vec<String> {
    let in_alloc = Allocator::default();
    let in_program = parse_program(&in_alloc, input).program;
    let out_alloc = Allocator::default();
    let out_program = parse_program(&out_alloc, output).program;
    let before = inferred_names(&in_program);
    let after = inferred_names(&out_program);
    let (in_starts, out_starts) = (line_starts(input), line_starts(output));
    let table = map.generate_lookup_table();
    let mut wrong = Vec::new();
    for (start, name) in &after {
        let (line, column) = position(output, &out_starts, *start as usize);
        let source = map
            .lookup_token(&table, line, column)
            .filter(|token| (token.get_dst_line(), token.get_dst_col()) == (line, column))
            .map(|token| (token.get_src_line(), token.get_src_col()));
        let Some(source) = source else {
            tally.untokened += 1;
            tally.unmapped += 1;
            continue;
        };
        let was = offset(input, &in_starts, source).and_then(|original| before.get(&original));
        let Some(was) = was else {
            tally.unmapped += 1;
            continue;
        };
        if *was == Inferred::Unknown || *name == Inferred::Unknown {
            continue;
        }
        tally.compared += 1;
        if was == name {
            continue;
        }
        wrong.push(format!(
            "{label}: input function at {}:{} had name {was:?}, output has {name:?}",
            source.0 + 1,
            source.1 + 1
        ));
    }
    wrong.sort();
    wrong
}

fn corpus() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for dir in EVERY_FILE {
        files.extend(js_files(&Path::new(FIXTURES).join(dir)));
    }
    for (dir, every) in SAMPLED {
        files.extend(
            js_files(&Path::new(FIXTURES).join(dir))
                .into_iter()
                .step_by(every),
        );
    }
    files
}

fn js_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(js_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "js") {
            found.push(path);
        }
    }
    found.sort();
    found
}

fn reads_function_names(text: &str) -> bool {
    let alloc = Allocator::default();
    let program = parse_program(&alloc, text).program;
    let scoping = build_scoping(&program).scoping;
    reads_names(&program, &scoping)
}

fn lane_differences(label: &str, options: &Options, input: &str, tally: &mut Tally) -> Vec<String> {
    let output = Pipeline::new(options.clone()).run(input);
    if let Some(bundle) = &output.bundle {
        tally.trees += 1;
        let mut wrong = Vec::new();
        for module in &bundle.modules {
            match &module.map {
                Some(map) => wrong.extend(differences(
                    &format!("{label} {}", module.path),
                    (input, &module.code),
                    map,
                    tally,
                )),
                None => tally.unmapped_modules += 1,
            }
        }
        return wrong;
    }
    if output.codes.len() != 1 {
        return Vec::new();
    }
    let Some(Some(map)) = output.source_maps.first() else {
        tally.unmapped_modules += 1;
        return Vec::new();
    };
    differences(label, (input, output.code()), map, tally)
}

struct Lane {
    name: &'static str,
    options: Options,
    renames: bool,
}

fn lanes() -> Vec<Lane> {
    let with = |rename| Options {
        rename,
        source_map: true,
        ..Options::default()
    };
    let lane = |name, rename| Lane {
        name,
        options: with(rename),
        renames: rename != RenameMode::None,
    };
    vec![
        lane("default", RenameMode::None),
        lane("rename smart", RenameMode::Smart),
        lane("rename mangle", RenameMode::Mangle),
        lane("rename simple", RenameMode::Simple),
        lane("rename unique", RenameMode::Unique),
    ]
}

fn lane_input(lane: &Lane, text: &str) -> Option<String> {
    if !lane.renames {
        return Some(text.to_string());
    }
    let input = format!("{text}{NAME_READ}");
    reads_function_names(&input).then_some(input)
}

#[test]
fn every_function_keeps_its_inferred_name_in_the_default_lane_and_where_rename_pins_it() {
    let files = corpus();
    assert!(files.len() > 300, "found {} fixtures", files.len());
    let mut wrong = Vec::new();
    let mut report = Vec::new();
    for lane in lanes() {
        let mut tally = Tally::default();
        for path in &files {
            let Ok(text) = std::fs::read_to_string(path) else {
                continue;
            };
            let Some(input) = lane_input(&lane, &text) else {
                continue;
            };
            let relative = path.strip_prefix(FIXTURES).unwrap_or(path).display();
            wrong.extend(lane_differences(
                &format!("{relative} ({})", lane.name),
                &lane.options,
                &input,
                &mut tally,
            ));
        }
        let seen = tally.compared + tally.unmapped;
        let share = tally.unmapped as f64 / seen.max(1) as f64;
        report.push(format!(
            "{}: compared {}, unmapped output functions {} ({:.1} %, {} with no token at their start), trees {}, outputs without a map {}",
            lane.name,
            tally.compared,
            tally.unmapped,
            share * 100.0,
            tally.untokened,
            tally.trees,
            tally.unmapped_modules
        ));
        assert!(tally.compared > 2000, "{}", report.join("\n"));
        assert!(share < UNMAPPED_CEILING, "{}", report.join("\n"));
        assert!(lane.renames || tally.trees > 0, "{}", report.join("\n"));
    }
    eprintln!("{}", report.join("\n"));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_check_sees_a_renamed_function() {
    let input = "var f =\nfunction () {};\nconsole.log(f.name);\n";
    let output = "var g =\nfunction () {};\nconsole.log(g.name);\n";
    let map = untwist_core::source_map::identity(input);
    let mut tally = Tally::default();
    let wrong = differences("probe", (input, output), &map, &mut tally);
    assert_eq!(tally.compared, 1);
    assert_eq!(wrong.len(), 1, "{wrong:?}");
    assert!(wrong[0].contains("\"f\""), "{wrong:?}");
}

const MODEL_ROWS: [&str; 14] = [
    "var a = function () {}; console.log(JSON.stringify(a.name));",
    "var b = class {}; console.log(JSON.stringify(b.name));",
    "var c = () => 0; console.log(JSON.stringify(c.name));",
    "var d; d = function () {}; console.log(JSON.stringify(d.name));",
    "var e; (e) = function () {}; console.log(JSON.stringify(e.name));",
    "var o = { p: function () {}, ['q']: () => 0, get r() { return 0; }, s() {} }; console.log(JSON.stringify([o.p.name, o.q.name, Object.getOwnPropertyDescriptor(o, 'r').get.name, o.s.name]));",
    "function f(g = function () {}) { return g.name; } console.log(JSON.stringify([f.name, f()]));",
    "var [h = class {}] = []; var { i = () => 0 } = {}; console.log(JSON.stringify([h.name, i.name]));",
    "var j = (0, function () {}); console.log(JSON.stringify(j.name));",
    "var k = { __proto__: function () {} }; console.log(JSON.stringify(Object.getPrototypeOf(k).name));",
    "class L { static m = function () {}; #n = () => 0; get n() { return this.#n; } } console.log(JSON.stringify([L.name, L.m.name, new L().n.name, Object.getOwnPropertyDescriptor(L.prototype, 'n').get.name]));",
    "var p; p ||= function () {}; var q = 1; q &&= class {}; var r; r ??= () => 0; console.log(JSON.stringify([p.name, q.name, r.name]));",
    "var s = { 1: function () {}, 'a b': class {} }; console.log(JSON.stringify([s[1].name, s['a b'].name]));",
    "var t = true ? function () {} : 0; var u = (1, function () {}); console.log(JSON.stringify([t.name, u.name]));",
];

fn model_names(text: &str) -> Vec<String> {
    let alloc = Allocator::default();
    let program = parse_program(&alloc, text).program;
    let mut found: Vec<(u32, Inferred)> = inferred_names(&program).into_iter().collect();
    found.sort_by_key(|(start, _)| *start);
    found
        .into_iter()
        .filter_map(|(_, name)| match name {
            Inferred::Name(name) => Some(name),
            Inferred::Unknown => None,
        })
        .collect()
}

#[test]
fn the_name_model_agrees_with_node() {
    if untwist_testing::node_rows::node_or_skip().is_none() {
        return;
    }
    let mut wrong = Vec::new();
    for row in MODEL_ROWS {
        let printed = untwist_testing::node::printed_by_node(row).unwrap_or_default();
        let model = model_names(row);
        let named: Vec<&String> = model.iter().filter(|name| !name.is_empty()).collect();
        let shown: Vec<String> = printed
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(',')
            .map(|part| part.trim_matches('"').to_string())
            .filter(|part| !part.is_empty())
            .collect();
        let shown: Vec<&String> = shown.iter().collect();
        if named != shown {
            wrong.push(format!(
                "{row}: node printed {printed:?}, the model gives {model:?}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
