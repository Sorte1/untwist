mod helpers;
mod names;
mod pollution;
mod scan;

use oxc_ast::ast::{AssignmentTarget, BindingPattern, Expression, Function, Statement};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashSet;

use crate::decode::{DecoderProgram, FnRef};
use crate::stack;

use super::collect::{
    rotator_function, self_rewrite_target, template_rewrite, Collected, FinalKey, TriggerKind,
};
use super::Owner;
use names::{FORBIDDEN_NAMES, INTRINSICS};
use scan::{Memo, Mode, Rules, Scanner};

fn memo_of<'a>(collected: &Collected<'a>) -> Result<Option<Memo<'a>>, ()> {
    let mut found: Option<Memo<'a>> = None;
    for trigger in &collected.triggers {
        let memo = match &trigger.kind {
            TriggerKind::Member(peel) => match (peel.segment, peel.final_key) {
                (Some(segment), FinalKey::Computed(_)) => Memo::Named(segment),
                _ => continue,
            },
            TriggerKind::CapturedMemo(peel, _) if peel.segment.is_none() => {
                Memo::Captured(peel.root)
            }
            TriggerKind::CapturedMemo(..) => return Err(()),
            _ => continue,
        };
        if found.is_some_and(|seen| seen != memo) {
            return Err(());
        }
        found = Some(memo);
    }
    Ok(found)
}

struct Own<'a> {
    names: FxHashSet<&'a str>,
}

impl<'a> VisitJs<'a> for Own<'a> {
    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_assignment_target(self, it));
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_binding_pattern(self, it));
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_binding_identifier(&mut self, it: &oxc_ast::ast::BindingIdentifier<'a>) {
        self.names.insert(it.name.as_str());
    }

    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _it: &oxc_ast::ast::ArrowFunctionExpression<'a>) {
    }
}

fn own_names<'a>(function: &Function<'a>) -> FxHashSet<&'a str> {
    let mut own = Own {
        names: FxHashSet::default(),
    };
    own.visit_formal_parameters(&function.params);
    if let Some(body) = &function.body {
        own.visit_function_body(body);
    }
    own.names
}

struct Finder<'s, 'a> {
    rules: &'s Rules<'s, 'a>,
    sensitive: &'s FxHashSet<&'a str>,
    pending: &'s FxHashSet<Span>,
    done: &'s mut FxHashSet<Span>,
    reached: FxHashSet<Span>,
    failed: bool,
}

impl<'a> Finder<'_, 'a> {
    fn scan(&mut self, function: &Function<'a>) {
        let declared = names::declared_in(function, &[]);
        if declared.iter().any(|name| self.sensitive.contains(name)) {
            self.failed = true;
            return;
        }
        let mut scanner = Scanner::new(self.rules, Mode::Helper, declared);
        walk_js::walk_function(&mut scanner, function, ScopeFlags::Function);
        self.failed |= scanner.failed;
        self.reached.extend(scanner.reached);
    }
}

impl<'a> VisitJs<'a> for Finder<'_, 'a> {
    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_assignment_target(self, it));
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_binding_pattern(self, it));
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if self.pending.contains(&it.span) && self.done.insert(it.span) {
            self.scan(it);
        }
        walk_js::walk_function(self, it, flags);
    }
}

fn helpers_clean<'a>(
    function: &Function<'a>,
    rules: &Rules<'_, 'a>,
    sensitive: &FxHashSet<&'a str>,
    start: FxHashSet<Span>,
) -> bool {
    let mut done = FxHashSet::default();
    let mut pending = start;
    while !pending.is_empty() {
        let mut finder = Finder {
            rules,
            sensitive,
            pending: &pending,
            done: &mut done,
            reached: FxHashSet::default(),
            failed: false,
        };
        finder.visit_function(function, ScopeFlags::Function);
        let reached = std::mem::take(&mut finder.reached);
        if finder.failed || pending.iter().any(|span| !done.contains(span)) {
            return false;
        }
        pending = reached.difference(&done).copied().collect();
    }
    true
}

fn memo_consistent(scanner: &Scanner<'_, '_>) -> bool {
    let [key] = scanner.writes.as_slice() else {
        return false;
    };
    !scanner.reads.is_empty()
        && scanner.reads.iter().all(|read| read == key)
        && !scanner.assigned.contains(key)
        && scanner.inits.get(key).copied().unwrap_or(0) <= 1
}

fn sensitive_names<'a>(
    machinery: &FxHashSet<&'a str>,
    outer: &FxHashSet<&'a str>,
    memo: Memo<'a>,
) -> FxHashSet<&'a str> {
    let mut sensitive: FxHashSet<&'a str> = machinery.union(outer).copied().collect();
    sensitive.extend(INTRINSICS);
    sensitive.extend(FORBIDDEN_NAMES);
    if let Memo::Captured(root) = memo {
        sensitive.insert(root);
    }
    sensitive
}

struct Decoder<'s, 'a> {
    decoder: &'s FnRef<'a>,
    collected: &'s Collected<'a>,
    memo: Memo<'a>,
    machinery: &'s FxHashSet<&'a str>,
    array: &'a str,
}

fn decoder_proven(target: &Decoder<'_, '_>) -> bool {
    let (function, name) = (target.decoder.function, target.decoder.name);
    let Some(body) = function.body.as_ref() else {
        return false;
    };
    let inner = template_rewrite(function, name)
        .and_then(|_| self_rewrite_target(&body.statements, Some(name)));
    let Some(inner) = inner else {
        return false;
    };
    let gates: Vec<Span> = target
        .collected
        .gates
        .iter()
        .map(|gate| gate.region)
        .collect();
    let flags = target
        .collected
        .gates
        .iter()
        .filter(|gate| gate.root == name && gate.segment.is_none())
        .map(|gate| gate.flag_key)
        .collect();
    let arrays = names::array_locals(function, target.array);
    let params = names::param_names(inner);
    let mut outer = names::outer_names(function);
    outer.retain(|name| !params.contains(name) || arrays.contains(name));
    let sensitive = sensitive_names(target.machinery, &outer, target.memo);
    if names::declared_in(inner, &[])
        .iter()
        .any(|declared| sensitive.contains(declared))
    {
        return false;
    }
    let helpers = helpers::helpers(function, name);
    let rules = Rules {
        decoder: name,
        machinery: target.machinery,
        flags: &flags,
        memo: target.memo,
        arrays: &arrays,
        helpers: &helpers,
        gates: &gates,
    };
    tail_proven(function, inner, &rules, sensitive)
}

fn tail_proven<'a>(
    function: &Function<'a>,
    inner: &Function<'a>,
    rules: &Rules<'_, 'a>,
    mut sensitive: FxHashSet<&'a str>,
) -> bool {
    let mut scanner = Scanner::new(rules, Mode::Tail, names::declared_in(inner, rules.gates));
    walk_js::walk_function(&mut scanner, inner, ScopeFlags::Function);
    if scanner.failed || !memo_consistent(&scanner) {
        return false;
    }
    sensitive.extend(own_names(inner));
    let reached = std::mem::take(&mut scanner.reached);
    helpers_clean(function, rules, &sensitive, reached)
}

fn session_functions<'a>(program: &DecoderProgram<'a>) -> Vec<&'a Function<'a>> {
    let mut functions = program.functions();
    functions.extend(program.rotator.and_then(rotator_function));
    functions
}

pub(super) fn proven<'a>(program: &DecoderProgram<'a>, owned: &[(Owner, Collected<'a>)]) -> bool {
    let machinery: FxHashSet<&'a str> = std::iter::once(program.string_array.name)
        .chain(program.decoders.iter().map(|decoder| decoder.name))
        .collect();
    if !pollution::clean(&session_functions(program), &machinery) {
        return false;
    }
    owned.iter().all(|(owner, collected)| {
        let memo = match (memo_of(collected), owner) {
            (Ok(None), _) => return true,
            (Ok(Some(memo)), Owner::Decoder(_)) => memo,
            _ => return false,
        };
        let Owner::Decoder(index) = owner else {
            return false;
        };
        decoder_proven(&Decoder {
            decoder: &program.decoders[*index],
            collected,
            memo,
            machinery: &machinery,
            array: program.string_array.name,
        })
    })
}
