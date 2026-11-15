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
const SAMPLED: [(&str, usize); 1] = [("untranspile", 16)];

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

fn differences(
    label: &str,
    (input, output): (&str, &str),
    map: &OwnedSourceMap,
    compared: &mut usize,
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
        let Some(token) = map.lookup_token(&table, line, column) else {
            continue;
        };
        if (token.get_dst_line(), token.get_dst_col()) != (line, column) {
            continue;
        }
        let source = (token.get_src_line(), token.get_src_col());
        let Some(original) = offset(input, &in_starts, source) else {
            continue;
        };
        let Some(was) = before.get(&original) else {
            continue;
        };
        if *was == Inferred::Unknown || *name == Inferred::Unknown {
            continue;
        }
        *compared += 1;
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

fn lane_differences(
    label: &str,
    options: &Options,
    input: &str,
    compared: &mut usize,
) -> Vec<String> {
    let output = Pipeline::new(options.clone()).run(input);
    if output.bundle.is_some() || output.codes.len() != 1 {
        return Vec::new();
    }
    let Some(Some(map)) = output.source_maps.first() else {
        return Vec::new();
    };
    differences(label, (input, output.code()), map, compared)
}

fn lanes() -> Vec<(&'static str, Options)> {
    let with = |rename| Options {
        rename,
        source_map: true,
        ..Options::default()
    };
    vec![
        ("default", with(RenameMode::None)),
        ("rename smart", with(RenameMode::Smart)),
        ("rename mangle", with(RenameMode::Mangle)),
    ]
}

#[test]
fn every_function_keeps_its_inferred_name_where_the_program_may_read_names() {
    let files = corpus();
    assert!(files.len() > 300, "found {} fixtures", files.len());
    let lanes = lanes();
    let mut wrong = Vec::new();
    let mut compared = 0;
    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let input = format!("{text}{NAME_READ}");
        if !reads_function_names(&input) {
            continue;
        }
        let relative = path.strip_prefix(FIXTURES).unwrap_or(path).display();
        for (lane, options) in &lanes {
            wrong.extend(lane_differences(
                &format!("{relative} ({lane})"),
                options,
                &input,
                &mut compared,
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert!(compared > 2000, "compared {compared} functions");
}

#[test]
fn the_check_sees_a_renamed_function() {
    let input = "var f =\nfunction () {};\nconsole.log(f.name);\n";
    let output = "var g =\nfunction () {};\nconsole.log(g.name);\n";
    let map = untwist_core::source_map::identity(input);
    let mut compared = 0;
    let wrong = differences("probe", (input, output), &map, &mut compared);
    assert_eq!(compared, 1);
    assert_eq!(wrong.len(), 1, "{wrong:?}");
    assert!(wrong[0].contains("\"f\""), "{wrong:?}");
}
