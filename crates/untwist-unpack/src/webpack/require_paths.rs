use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_span::GetSpan;
use oxc_str::{Ident, Str};
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::prop::{identifier_name, number_key};

use crate::matcher::argument_expression;
use crate::model::Bundle;
use crate::path::relative_path;
use crate::webpack::shadow;

fn literal_module_id(argument: Option<&Argument<'_>>) -> Option<String> {
    match argument_expression(argument?)? {
        Expression::NumericLiteral(literal) => Some(number_key(literal.value)),
        Expression::StringLiteral(literal) if !literal.lone_surrogates => {
            Some(literal.value.as_str().to_string())
        }
        _ => None,
    }
}

fn require_argument_id(call: &CallExpression<'_>) -> Option<String> {
    if call.optional || call.arguments.len() != 1 {
        return None;
    }
    if identifier_name(&call.callee) != Some("require") {
        return None;
    }
    literal_module_id(call.arguments.first())
}

fn bound_require_id(call: &CallExpression<'_>) -> Option<(String, u32)> {
    if call.optional || call.arguments.len() != 2 {
        return None;
    }
    let object = crate::matcher::const_member(&call.callee, "bind")?;
    if identifier_name(object) != Some("require") {
        return None;
    }
    let bound_null = matches!(
        call.arguments.first().and_then(argument_expression),
        Some(Expression::NullLiteral(_))
    ) || call
        .arguments
        .first()
        .and_then(argument_expression)
        .and_then(identifier_name)
        == Some("require");
    let id = literal_module_id(call.arguments.get(1))?;
    bound_null.then_some((id, object.span().start))
}

struct Rewrite<'a, 'r> {
    builder: AstBuilder<'a>,
    paths: &'r FxHashMap<String, String>,
    from: &'r str,
    missing: Vec<String>,
    converted: FxHashSet<u32>,
    bind: bool,
    track: bool,
    route: Option<(&'r str, &'r [String])>,
    shadowed: u32,
    sloppy: bool,
    helpers: Vec<String>,
}

pub struct Routing {
    pub available: Vec<String>,
    pub bindings: Vec<String>,
}

pub const RUNTIME_HELPERS: [&str; 5] = ["d", "n", "o", "r", "t"];

impl<'a, 'r> Rewrite<'a, 'r> {
    fn scoped(&mut self, shadows: bool, walk: impl FnOnce(&mut Self)) {
        let shadows = u32::from(shadows);
        self.shadowed += shadows;
        walk(self);
        self.shadowed -= shadows;
    }

    #[cold]
    fn routed(&mut self, it: &StaticMemberExpression<'a>) -> Option<&'r str> {
        let (binding, available) = self.route.filter(|_| self.bind && self.shadowed == 0)?;
        if identifier_name(&it.object) != Some("require") {
            return None;
        }
        let name = it.property.name.as_str();
        available.iter().find(|helper| helper.as_str() == name)?;
        if !self.helpers.iter().any(|helper| helper == name) {
            self.helpers.push(name.to_string());
        }
        Some(binding)
    }

    fn bound_this(&mut self, it: &mut CallExpression<'a>) {
        let Some(binding) = self
            .route
            .filter(|_| self.bind && self.shadowed == 0)
            .map(|r| r.0)
        else {
            return;
        };
        let Some(member) = crate::matcher::const_member(&it.callee, "bind") else {
            return;
        };
        let routed = matches!(member, Expression::StaticMemberExpression(inner)
            if identifier_name(&inner.object) == Some("require")
                && self.route.is_some_and(|(_, available)| available.iter().any(|h| h == inner.property.name.as_str())));
        if !routed {
            return;
        }
        if let Some(Argument::Identifier(this)) = it.arguments.first_mut() {
            if this.name == "require" {
                this.name = Ident::from_str_in(binding, &self.builder);
            }
        }
    }

    fn replace(&mut self, argument: Option<&mut Argument<'a>>, module_id: &str) {
        let specifier = self.specifier(module_id);
        if let Some(argument) = argument {
            let span = argument.span();
            *argument = Argument::from(Expression::new_string_literal(
                span,
                Str::from_str_in(&specifier, &self.builder),
                None,
                &self.builder,
            ));
        }
    }

    fn specifier(&mut self, module_id: &str) -> String {
        match self.paths.get(module_id) {
            Some(path) => relative_path(self.from, path),
            None => {
                self.missing.push(module_id.to_string());
                relative_path(self.from, &format!("{module_id}.js"))
            }
        }
    }
}

impl<'a> VisitMut<'a> for Rewrite<'a, '_> {
    fn visit_call_expression(&mut self, it: &mut CallExpression<'a>) {
        if self.bind {
            if let Some((form, mode)) = crate::webpack::require_t::form(it) {
                if mode == crate::webpack::require_t::PASS_THROUGH {
                    crate::webpack::require_t::pass_through(&self.builder, it, &form);
                }
            }
        }
        if let Some(module_id) = require_argument_id(it) {
            if self.track {
                self.converted.insert(it.callee.span().start);
            }
            self.replace(it.arguments.first_mut(), &module_id);
        } else if let Some((module_id, start)) = self.bind.then(|| bound_require_id(it)).flatten() {
            if self.track {
                self.converted.insert(start);
            }
            self.replace(it.arguments.get_mut(1), &module_id);
        }
        self.bound_this(it);
        walk_mut::walk_call_expression(self, it);
    }

    #[inline]
    fn visit_static_member_expression(&mut self, it: &mut StaticMemberExpression<'a>) {
        if let Some(binding) = self.route.and_then(|_| self.routed(it)) {
            if let Expression::Identifier(object) = &mut it.object {
                object.name = Ident::from_str_in(binding, &self.builder);
            }
            return;
        }
        walk_mut::walk_static_member_expression(self, it);
    }

    #[inline]
    fn visit_program(&mut self, it: &mut Program<'a>) {
        if self.route.is_none() {
            return walk_mut::walk_program(self, it);
        }
        self.sloppy = !shadow::strict(&it.directives) && !it.source_type.is_module();
        let shadows = self.route.is_some()
            && (shadow::hoisted(&it.body, self.sloppy) || shadow::lexical(&it.body));
        self.scoped(shadows, |this| walk_mut::walk_program(this, it));
    }

    #[inline]
    fn visit_function(&mut self, it: &mut Function<'a>, flags: oxc_syntax::scope::ScopeFlags) {
        if self.route.is_none() {
            return walk_mut::walk_function(self, it, flags);
        }
        let sloppy = self.sloppy;
        if let Some(body) = &it.body {
            self.sloppy &= !shadow::strict(&body.directives);
        }
        let shadows = self.route.is_some()
            && (shadow::params_bind(&it.params)
                || it.body.as_ref().is_some_and(|body| {
                    shadow::hoisted(&body.statements, self.sloppy)
                        || shadow::lexical(&body.statements)
                }));
        self.scoped(shadows, |this| walk_mut::walk_function(this, it, flags));
        self.sloppy = sloppy;
    }

    #[inline]
    fn visit_arrow_function_expression(&mut self, it: &mut ArrowFunctionExpression<'a>) {
        if self.route.is_none() {
            return walk_mut::walk_arrow_function_expression(self, it);
        }
        let body = match &it.body {
            ArrowFunctionBody::FunctionBody(body) => {
                shadow::hoisted(&body.statements, self.sloppy) || shadow::lexical(&body.statements)
            }
            _ => false,
        };
        let shadows = self.route.is_some() && (shadow::params_bind(&it.params) || body);
        self.scoped(shadows, |this| {
            walk_mut::walk_arrow_function_expression(this, it)
        });
    }

    #[inline]
    fn visit_block_statement(&mut self, it: &mut BlockStatement<'a>) {
        if self.route.is_none() {
            return walk_mut::walk_block_statement(self, it);
        }
        let shadows = self.route.is_some() && shadow::lexical(&it.body);
        self.scoped(shadows, |this| walk_mut::walk_block_statement(this, it));
    }

    #[inline]
    fn visit_static_block(&mut self, it: &mut StaticBlock<'a>) {
        if self.route.is_none() {
            return walk_mut::walk_static_block(self, it);
        }
        let shadows = self.route.is_some()
            && (shadow::lexical(&it.body) || shadow::hoisted(&it.body, self.sloppy));
        self.scoped(shadows, |this| walk_mut::walk_static_block(this, it));
    }

    #[inline]
    fn visit_switch_statement(&mut self, it: &mut SwitchStatement<'a>) {
        if self.route.is_none() {
            return walk_mut::walk_switch_statement(self, it);
        }
        let shadows = self.route.is_some()
            && it
                .cases
                .iter()
                .any(|case| shadow::lexical(&case.consequent));
        self.scoped(shadows, |this| walk_mut::walk_switch_statement(this, it));
    }

    #[inline]
    fn visit_catch_clause(&mut self, it: &mut CatchClause<'a>) {
        if self.route.is_none() {
            return walk_mut::walk_catch_clause(self, it);
        }
        let shadows = self.route.is_some()
            && it
                .param
                .as_ref()
                .is_some_and(|param| shadow::binds(&param.pattern));
        self.scoped(shadows, |this| walk_mut::walk_catch_clause(this, it));
    }

    #[inline]
    fn visit_for_statement(&mut self, it: &mut ForStatement<'a>) {
        if self.route.is_none() {
            return walk_mut::walk_for_statement(self, it);
        }
        let head = match &it.init {
            Some(ForStatementInit::VariableDeclaration(d)) => Some(&**d),
            _ => None,
        };
        let shadows = self.route.is_some() && shadow::for_head_lexical(head);
        self.scoped(shadows, |this| walk_mut::walk_for_statement(this, it));
    }

    #[inline]
    fn visit_for_in_statement(&mut self, it: &mut ForInStatement<'a>) {
        if self.route.is_none() {
            return walk_mut::walk_for_in_statement(self, it);
        }
        let head = match &it.left {
            ForStatementLeft::VariableDeclaration(d) => Some(&**d),
            _ => None,
        };
        let shadows = self.route.is_some() && shadow::for_head_lexical(head);
        self.scoped(shadows, |this| walk_mut::walk_for_in_statement(this, it));
    }

    #[inline]
    fn visit_for_of_statement(&mut self, it: &mut ForOfStatement<'a>) {
        if self.route.is_none() {
            return walk_mut::walk_for_of_statement(self, it);
        }
        let head = match &it.left {
            ForStatementLeft::VariableDeclaration(d) => Some(&**d),
            _ => None,
        };
        let shadows = self.route.is_some() && shadow::for_head_lexical(head);
        self.scoped(shadows, |this| walk_mut::walk_for_of_statement(this, it));
    }

    fn visit_import_declaration(&mut self, it: &mut ImportDeclaration<'a>) {
        if !it.source.lone_surrogates {
            let specifier = self.specifier(it.source.value.as_str());
            let span = it.source.span;
            it.source = StringLiteral::new(
                span,
                Str::from_str_in(&specifier, &self.builder),
                None,
                &self.builder,
            );
        }
        walk_mut::walk_import_declaration(self, it);
    }
}

#[derive(Default)]
pub struct Rewritten {
    pub converted: FxHashSet<u32>,
    pub helpers: Vec<String>,
}

pub type Converted = Vec<(usize, Rewritten)>;

pub fn rewritten(converted: &Converted, index: usize) -> Option<&Rewritten> {
    converted
        .iter()
        .find(|(at, _)| *at == index)
        .map(|(_, rewritten)| rewritten)
}

pub fn replace_require_paths<'a>(
    alloc: &'a Allocator,
    bundle: &mut Bundle<'a>,
) -> (Vec<(String, String)>, Converted) {
    replace_require_paths_with(alloc, bundle, true, &[], None)
}

pub fn replace_require_paths_with<'a>(
    alloc: &'a Allocator,
    bundle: &mut Bundle<'a>,
    bind: bool,
    tracked: &[bool],
    routing: Option<&Routing>,
) -> (Vec<(String, String)>, Converted) {
    let paths: FxHashMap<String, String> = bundle
        .modules
        .iter()
        .map(|(id, module)| (id.clone(), module.path.as_str().to_string()))
        .collect();
    let mut missing = Vec::new();
    let mut converted = Converted::new();
    for (index, (id, module)) in bundle.modules.iter_mut().enumerate() {
        let from = module.path.as_str().to_string();
        let mut rewrite = Rewrite {
            builder: AstBuilder::new(alloc),
            paths: &paths,
            from: &from,
            missing: Vec::new(),
            converted: FxHashSet::default(),
            bind,
            track: tracked.get(index).copied().unwrap_or(false),
            route: routing.and_then(|routing| {
                Some((
                    routing.bindings.get(index)?.as_str(),
                    routing.available.as_slice(),
                ))
            }),
            shadowed: 0,
            sloppy: false,
            helpers: Vec::new(),
        };
        rewrite.visit_program(module.program_mut());
        for target in rewrite.missing {
            missing.push((id.clone(), target));
        }
        if !rewrite.helpers.is_empty() || !rewrite.converted.is_empty() {
            converted.push((
                index,
                Rewritten {
                    converted: rewrite.converted,
                    helpers: rewrite.helpers,
                },
            ));
        }
    }
    (missing, converted)
}
