use oxc_ast::ast::{BindingPattern, IdentifierReference, Program, Statement};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;

use super::names::{mark_spelled, scan_names, NameScan, Write};

#[derive(Default)]
pub struct TextWatch<'a> {
    names: Vec<&'a str>,
    min_len: usize,
    spelled: Vec<Option<Span>>,
    writes: Vec<Write>,
}

pub struct Watched {
    pub spelled: Vec<Option<Span>>,
    pub writes: Vec<Write>,
}

pub fn declared_text<'a>(stmt: &Statement<'a>) -> &'a str {
    let id = match stmt {
        Statement::FunctionDeclaration(function) => function.id.as_ref(),
        Statement::VariableDeclaration(declaration) => match declaration.declarations.as_slice() {
            [declarator] => match &declarator.id {
                BindingPattern::BindingIdentifier(id) => Some(id.as_ref()),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    };
    id.map_or("", |id| id.name.as_str())
}

pub fn reachable_by_direct_eval(scoping: &Scoping, symbol: SymbolId) -> bool {
    let scope = scoping.symbol_scope_id(symbol);
    scoping.scope_flags(scope).contains(ScopeFlags::DirectEval)
}

fn slot_of(names: &[&str], name: &str) -> Option<usize> {
    names.iter().position(|known| *known == name)
}

fn copy_writes(from: &[Write], at: usize, index: usize, into: &mut Vec<Write>) {
    into.extend(
        from.iter()
            .filter(|write| write.index == at)
            .map(|write| Write { index, ..*write }),
    );
}

impl<'a> TextWatch<'a> {
    pub fn over(names: Vec<&'a str>) -> Self {
        let min_len = names.iter().map(|name| name.len()).min().unwrap_or(0);
        let spelled = vec![None; names.len()];
        Self {
            names,
            min_len,
            spelled,
            writes: Vec::new(),
        }
    }

    pub fn text(&mut self, text: &str, span: Span) {
        if self.names.is_empty() || text.len() < self.min_len {
            return;
        }
        mark_spelled(&mut self.spelled, &self.names, text, span);
    }

    pub fn assigned(&mut self, id: &IdentifierReference<'_>) {
        if self.names.is_empty() {
            return;
        }
        let Some(index) = slot_of(&self.names, id.name.as_str()) else {
            return;
        };
        if let Some(reference) = id.reference_id.get() {
            self.writes.push(Write {
                index,
                reference,
                span: id.span,
            });
        }
    }

    pub fn resolve(self, program: &Program<'_>, found: &[&str], reachable: &[bool]) -> Watched {
        let missing: Vec<&str> = found
            .iter()
            .copied()
            .filter(|name| slot_of(&self.names, name).is_none())
            .collect();
        let scanned = if missing.is_empty() {
            NameScan::default()
        } else {
            scan_names(program, &missing)
        };
        let mut watched = Watched {
            spelled: Vec::with_capacity(found.len()),
            writes: Vec::new(),
        };
        for (index, name) in found.iter().enumerate() {
            let (spelled, writes, at) = match slot_of(&self.names, name) {
                Some(at) => (&self.spelled, &self.writes, at),
                None => (
                    &scanned.spelled,
                    &scanned.writes,
                    slot_of(&missing, name).unwrap_or(usize::MAX),
                ),
            };
            let shown = reachable.get(index).copied().unwrap_or(true);
            let span = spelled.get(at).copied().flatten().filter(|_| shown);
            watched.spelled.push(span);
            copy_writes(writes, at, index, &mut watched.writes);
        }
        watched
    }
}

macro_rules! watch_texts {
    () => {
        fn visit_string_literal(&mut self, it: &oxc_ast::ast::StringLiteral<'a>) {
            if WATCHING {
                self.watch.text(it.value.as_str(), it.span);
            }
        }

        fn visit_template_element(&mut self, it: &oxc_ast::ast::TemplateElement<'a>) {
            if WATCHING {
                self.watch.text(it.value.raw.as_str(), it.span);
            }
        }

        fn visit_reg_exp_literal(&mut self, it: &oxc_ast::ast::RegExpLiteral<'a>) {
            if WATCHING {
                self.watch.text(
                    &$crate::proof::regex_name_text(it.regex.pattern.text.as_str()),
                    it.span,
                );
            }
        }

        fn visit_static_member_expression(
            &mut self,
            it: &oxc_ast::ast::StaticMemberExpression<'a>,
        ) {
            if WATCHING {
                self.watch.text(it.property.name.as_str(), it.property.span);
            }
            oxc_ast_visit::walk_js::walk_static_member_expression(self, it);
        }

        fn visit_object_property(&mut self, it: &oxc_ast::ast::ObjectProperty<'a>) {
            if WATCHING && !it.computed {
                self.visit_expression(&it.value);
            } else {
                oxc_ast_visit::walk_js::walk_object_property(self, it);
            }
        }

        fn visit_property_key(&mut self, it: &oxc_ast::ast::PropertyKey<'a>) {
            if let (true, oxc_ast::ast::PropertyKey::StaticIdentifier(id)) = (WATCHING, it) {
                self.watch.text(id.name.as_str(), id.span);
            }
            oxc_ast_visit::walk_js::walk_property_key(self, it);
        }

        fn visit_simple_assignment_target(
            &mut self,
            it: &oxc_ast::ast::SimpleAssignmentTarget<'a>,
        ) {
            if let (true, oxc_ast::ast::SimpleAssignmentTarget::AssignmentTargetIdentifier(id)) =
                (WATCHING, it)
            {
                self.watch.assigned(id);
            }
            oxc_ast_visit::walk_js::walk_simple_assignment_target(self, it);
        }
    };
}

pub(crate) use watch_texts;
