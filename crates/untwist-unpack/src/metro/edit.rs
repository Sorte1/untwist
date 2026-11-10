use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_str::{Ident, Str};
use oxc_syntax::number::NumberBase;
use rustc_hash::FxHashSet;

use super::layout::module_id;
use super::{DEPENDENCY_MAP_NAME, GLOBAL_NAME, IMPORT_ALL_NAME, IMPORT_DEFAULT_NAME};
use crate::matcher::argument_expression;
use crate::specifier::ModulePaths;

#[derive(Default)]
pub struct Used {
    pub require: Option<String>,
    pub import_default: bool,
    pub import_all: bool,
    pub prelude: FxHashSet<String>,
    pub global: bool,
    pub dependency_map: bool,
}

impl Used {
    pub fn helpers(&self) -> bool {
        self.require.is_some() || self.import_default || self.import_all
    }
}

pub fn text_fresh(text: &str, prelude: &[String]) -> String {
    (1..)
        .map(|index| match index {
            1 => String::from("_require"),
            index => format!("_require{index}"),
        })
        .find(|name| !text.contains(name.as_str()) && !prelude.contains(name))
        .unwrap_or_default()
}

pub struct Edit<'a, 'r> {
    pub builder: AstBuilder<'a>,
    pub scoping: Option<&'r Scoping>,
    pub checking: bool,
    pub shadowed: bool,
    pub escaped: bool,
    pub deps: &'r [(String, String)],
    pub paths: &'r ModulePaths,
    pub from: &'r str,
    pub prelude: &'r [String],
    pub fresh: Option<Ident<'a>>,
    pub text: &'r str,
    pub used: Used,
    pub missing: Vec<String>,
}

impl<'a> Edit<'a, '_> {
    fn free(&self, it: &IdentifierReference<'_>) -> bool {
        crate::scan::is_free(self.scoping, it)
    }

    fn watched(&self, name: &str) -> bool {
        matches!(
            name,
            "require" | IMPORT_DEFAULT_NAME | IMPORT_ALL_NAME | GLOBAL_NAME | DEPENDENCY_MAP_NAME
        ) || self.prelude.iter().any(|p| p == name)
    }

    fn direct_require(&self, call: &CallExpression<'_>) -> bool {
        let Expression::Identifier(callee) = &call.callee else {
            return false;
        };
        let argument = call.arguments.first().and_then(argument_expression);
        callee.name == "require"
            && call.arguments.len() == 1
            && argument
                .and_then(|argument| self.required_id(argument))
                .is_some_and(|id| self.paths.contains_key(&id))
    }

    fn check_call(&mut self, it: &mut CallExpression<'a>) {
        if self.direct_require(it) {
            for argument in it.arguments.iter_mut() {
                self.visit_argument(argument);
            }
        } else {
            walk_mut::walk_call_expression(self, it);
        }
    }

    fn indexed(&self, member: &ComputedMemberExpression<'_>) -> Option<&(String, String)> {
        let Expression::Identifier(object) = member.object.without_parentheses() else {
            return None;
        };
        if member.optional || object.name != DEPENDENCY_MAP_NAME || !self.free(object) {
            return None;
        }
        let Expression::NumericLiteral(index) = member.expression.without_parentheses() else {
            return None;
        };
        if index.value.fract() != 0.0 || index.value < 0.0 {
            return None;
        }
        self.deps.get(index.value as usize)
    }

    fn literal(&self, span: Span, (text, id): &(String, String)) -> Option<Expression<'a>> {
        Some(match !text.starts_with('"') {
            true => Expression::new_numeric_literal(
                span,
                id.parse().ok()?,
                None,
                NumberBase::Decimal,
                &self.builder,
            ),
            false => Expression::new_string_literal(
                span,
                Str::from_str_in(id, &self.builder),
                None,
                &self.builder,
            ),
        })
    }

    fn required_id(&self, argument: &Expression<'_>) -> Option<String> {
        match argument.without_parentheses() {
            Expression::ComputedMemberExpression(member) => {
                self.indexed(member).map(|(_, id)| id.clone())
            }
            other => module_id(other),
        }
    }

    fn required_path(&mut self, call: &CallExpression<'a>) -> Option<String> {
        let Expression::Identifier(callee) = &call.callee else {
            return None;
        };
        if callee.name != "require" || !self.free(callee) || call.arguments.len() != 1 {
            return None;
        }
        let id = self.required_id(argument_expression(call.arguments.first()?)?)?;
        match self.paths.get(&id) {
            Some(path) => Some(self.paths.specifier(self.from, path)),
            None => {
                self.missing.push(id);
                None
            }
        }
    }
}

impl<'a> VisitMut<'a> for Edit<'a, '_> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if self.checking {
            walk_mut::walk_expression(self, it);
            return;
        }
        if let Expression::ComputedMemberExpression(member) = it {
            if let Some(literal) = self
                .indexed(member)
                .and_then(|dep| self.literal(member.span, dep))
            {
                *it = literal;
                return;
            }
        }
        walk_mut::walk_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &mut CallExpression<'a>) {
        if self.checking {
            self.check_call(it);
            return;
        }
        let Some(specifier) = self.required_path(it) else {
            walk_mut::walk_call_expression(self, it);
            return;
        };
        if let Some(argument) = it.arguments.first_mut() {
            let span = argument.span();
            *argument = Argument::from(Expression::new_string_literal(
                span,
                Str::from_str_in(&specifier, &self.builder),
                None,
                &self.builder,
            ));
        }
    }

    fn visit_binding_identifier(&mut self, it: &mut BindingIdentifier<'a>) {
        self.shadowed |= self.checking && self.watched(it.name.as_str());
    }

    fn visit_identifier_reference(&mut self, it: &mut IdentifierReference<'a>) {
        if self.checking {
            self.escaped |= it.name == "require";
            return;
        }
        if !self.free(it) {
            return;
        }
        match it.name.as_str() {
            "require" => {
                let fresh = match self.fresh {
                    Some(fresh) => fresh,
                    None => {
                        let name = text_fresh(self.text, self.prelude);
                        let fresh = Ident::from_str_in(&name, &self.builder);
                        self.fresh = Some(fresh);
                        fresh
                    }
                };
                self.used.require = Some(fresh.to_string());
                it.name = fresh;
            }
            IMPORT_DEFAULT_NAME => self.used.import_default = true,
            IMPORT_ALL_NAME => self.used.import_all = true,
            GLOBAL_NAME => self.used.global = true,
            DEPENDENCY_MAP_NAME => self.used.dependency_map = true,
            name if self.prelude.iter().any(|p| p == name) => {
                self.used.prelude.insert(name.to_string());
            }
            _ => {}
        }
    }
}
