use oxc_allocator::{Allocator, ReplaceWith};
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_str::Ident;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::prop::{identifier_name, number_key};

use crate::matcher::{argument_expression, call_parts, const_member};
use crate::model::{Bundle, Module};

fn free_require(scoping: Option<&Scoping>, expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::Identifier(id) if id.name == "require" && crate::scan::is_free(scoping, id))
}

fn require_call_id(scoping: Option<&Scoping>, expr: &Expression<'_>) -> Option<String> {
    let (callee, arguments) = call_parts(expr)?;
    if !free_require(scoping, callee) || arguments.len() != 1 {
        return None;
    }
    let Expression::NumericLiteral(literal) = argument_expression(arguments.first()?)? else {
        return None;
    };
    Some(number_key(literal.value))
}

fn require_n_argument<'k>(scoping: Option<&Scoping>, expr: &'k Expression<'_>) -> Option<&'k str> {
    let (callee, arguments) = call_parts(expr)?;
    let object = const_member(callee, "n")?;
    if !free_require(scoping, object) || arguments.len() != 1 {
        return None;
    }
    identifier_name(argument_expression(arguments.first()?)?)
}

fn interop_access<'k>(scoping: Option<&Scoping>, expr: &'k Expression<'_>) -> Option<&'k str> {
    if let Some(object) = const_member(expr, "a") {
        if let Some(name) = require_n_argument(scoping, object) {
            return Some(name);
        }
    }
    let (callee, arguments) = call_parts(expr)?;
    if !arguments.is_empty() {
        return None;
    }
    require_n_argument(scoping, callee)
}

struct RequireIds<'r> {
    scoping: Option<&'r Scoping>,
    ids: FxHashMap<String, String>,
}

impl<'a> VisitJs<'a> for RequireIds<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            if let Some(module_id) = require_call_id(self.scoping, init) {
                self.ids
                    .entry(id.name.as_str().to_string())
                    .or_insert(module_id);
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }
}

struct Interop<'a, 'r> {
    builder: AstBuilder<'a>,
    scoping: Option<&'r Scoping>,
    require_ids: &'r FxHashMap<String, String>,
    esm: &'r FxHashMap<String, bool>,
    getters: FxHashSet<String>,
    changes: usize,
}

impl<'a> Interop<'a, '_> {
    fn target(&self, name: &str, span: Span) -> Expression<'a> {
        let object = Expression::new_identifier(
            span,
            Ident::from_str_in(name, &self.builder),
            &self.builder,
        );
        let is_esm = self
            .require_ids
            .get(name)
            .and_then(|module_id| self.esm.get(module_id))
            .copied()
            .unwrap_or(false);
        if !is_esm {
            return object;
        }
        Expression::new_static_member_expression(
            span,
            object,
            IdentifierName::new(
                span,
                Ident::from_str_in("default", &self.builder),
                &self.builder,
            ),
            false,
            &self.builder,
        )
    }
}

impl<'a> VisitMut<'a> for Interop<'a, '_> {
    fn visit_variable_declarator(&mut self, it: &mut VariableDeclarator<'a>) {
        let matched = match (&it.id, it.init.as_ref()) {
            (BindingPattern::BindingIdentifier(id), Some(init)) => {
                require_n_argument(self.scoping, init)
                    .map(|name| (id.name.as_str().to_string(), name.to_string()))
            }
            _ => None,
        };
        if let Some((getter, name)) = matched {
            let span = it.span;
            it.init = Some(self.target(&name, span));
            self.getters.insert(getter);
            self.changes += 1;
        }
        walk_mut::walk_variable_declarator(self, it);
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if let Some(name) = interop_access(self.scoping, it).map(str::to_string) {
            let span = it.span();
            *it = self.target(&name, span);
            self.changes += 1;
        }
        walk_mut::walk_expression(self, it);
    }
}

struct Unwrap<'r> {
    getters: &'r FxHashSet<String>,
    changes: usize,
}

impl Unwrap<'_> {
    fn is_getter(&self, expr: &Expression<'_>) -> bool {
        identifier_name(expr).is_some_and(|name| self.getters.contains(name))
    }
}

impl<'a> VisitMut<'a> for Unwrap<'_> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        let collapse = match &*it {
            Expression::StaticMemberExpression(member) => self.is_getter(&member.object),
            Expression::ComputedMemberExpression(member) => self.is_getter(&member.object),
            Expression::CallExpression(call) => self.is_getter(&call.callee),
            _ => false,
        };
        if collapse {
            it.replace_with(|expr| match expr {
                Expression::StaticMemberExpression(member) => member.unbox().object,
                Expression::ComputedMemberExpression(member) => member.unbox().object,
                Expression::CallExpression(call) => call.unbox().callee,
                other => other,
            });
            self.changes += 1;
        }
        walk_mut::walk_expression(self, it);
    }
}

fn convert_module<'a>(
    alloc: &'a Allocator,
    module: &mut Module<'a>,
    esm: &FxHashMap<String, bool>,
) -> usize {
    let scoping = crate::webpack::require_paths::scoping_if_bound(module.program());
    let mut ids = RequireIds {
        scoping: scoping.as_ref(),
        ids: FxHashMap::default(),
    };
    ids.visit_program(module.program());
    let mut interop = Interop {
        builder: AstBuilder::new(alloc),
        scoping: scoping.as_ref(),
        require_ids: &ids.ids,
        esm,
        getters: FxHashSet::default(),
        changes: 0,
    };
    interop.visit_program(module.program_mut());
    let changes = interop.changes;
    let getters = interop.getters;
    if getters.is_empty() {
        return changes;
    }
    let mut unwrap = Unwrap {
        getters: &getters,
        changes: 0,
    };
    unwrap.visit_program(module.program_mut());
    changes + unwrap.changes
}

pub fn convert_default_require<'a>(alloc: &'a Allocator, bundle: &mut Bundle<'a>) -> usize {
    let esm: FxHashMap<String, bool> = bundle
        .modules
        .iter()
        .map(|(id, module)| (id.clone(), module.is_esm()))
        .collect();
    let mut changes = 0usize;
    for module in bundle.modules.values_mut() {
        changes += convert_module(alloc, module, &esm);
    }
    changes
}
