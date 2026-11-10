use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_str::{Ident, Str};
use rustc_hash::FxHashSet;
use untwist_core::ast_util::prop::number_key;
use untwist_core::semantic::build_scoping;

use crate::matcher::argument_expression;
use crate::model::Bundle;
use crate::specifier::ModulePaths;
use crate::webpack::shadow;

fn literal_module_id(argument: Option<&Argument<'_>>) -> Option<String> {
    match argument_expression(argument?)? {
        Expression::NumericLiteral(literal) => Some(number_key(literal.value)),
        Expression::StringLiteral(literal) => literal.value.as_str().map(str::to_string),
        _ => None,
    }
}

const REQUIRE: &str = "require";

struct Rewrite<'a, 'r> {
    builder: AstBuilder<'a>,
    paths: &'r ModulePaths,
    from: &'r str,
    missing: Vec<String>,
    converted: FxHashSet<u32>,
    bind: bool,
    track: bool,
    route: Option<(&'r str, &'r FxHashSet<String>)>,
    scoping: Option<Scoping>,
    shadowed: u32,
    sloppy: bool,
    helpers: Vec<String>,
}

pub struct Routing {
    pub available: FxHashSet<String>,
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

    fn free_require(&self, expr: &Expression<'_>) -> bool {
        match expr {
            Expression::Identifier(id) => self.free_reference(id),
            _ => false,
        }
    }

    fn free_reference(&self, id: &IdentifierReference<'_>) -> bool {
        id.name == REQUIRE && self.shadowed == 0 && crate::scan::is_free(self.scoping.as_ref(), id)
    }

    fn require_argument_id(&self, call: &CallExpression<'_>) -> Option<String> {
        if call.optional || call.arguments.len() != 1 || !self.free_require(&call.callee) {
            return None;
        }
        literal_module_id(call.arguments.first())
    }

    fn bound_require_id(&self, call: &CallExpression<'_>) -> Option<(String, u32)> {
        if call.optional || call.arguments.len() != 2 {
            return None;
        }
        let object = crate::matcher::const_member(&call.callee, "bind")?;
        if !self.free_require(object) {
            return None;
        }
        let this = call.arguments.first().and_then(argument_expression)?;
        let bound = matches!(this, Expression::NullLiteral(_)) || self.free_require(this);
        let id = literal_module_id(call.arguments.get(1))?;
        bound.then_some((id, object.span().start))
    }

    fn pass_through(&mut self, it: &mut CallExpression<'a>) {
        let Some((form, mode)) = crate::webpack::require_t::form(it) else {
            return;
        };
        let free = crate::webpack::require_t::subject(it, &form)
            .is_some_and(|subject| self.free_require(subject));
        if free && mode == crate::webpack::require_t::PASS_THROUGH {
            crate::webpack::require_t::pass_through(&self.builder, it, &form);
        }
    }

    #[cold]
    fn routed(&mut self, it: &StaticMemberExpression<'a>) -> Option<&'r str> {
        let (binding, available) = self.route.filter(|_| self.bind)?;
        if !self.free_require(&it.object) {
            return None;
        }
        let name = it.property.name.as_str();
        if !available.contains(name) {
            return None;
        }
        if !self.helpers.iter().any(|helper| helper == name) {
            self.helpers.push(name.to_string());
        }
        Some(binding)
    }

    fn bound_this(&mut self, it: &mut CallExpression<'a>) {
        let Some((binding, available)) = self.route.filter(|_| self.bind) else {
            return;
        };
        let Some(member) = crate::matcher::const_member(&it.callee, "bind") else {
            return;
        };
        let routed = matches!(member, Expression::StaticMemberExpression(inner)
            if self.free_require(&inner.object)
                && available.contains(inner.property.name.as_str()));
        let this = matches!(it.arguments.first(), Some(Argument::Identifier(this)) if self.free_reference(this));
        if !routed || !this {
            return;
        }
        if let Some(Argument::Identifier(this)) = it.arguments.first_mut() {
            this.name = Ident::from_str_in(binding, &self.builder);
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
            Some(path) => self.paths.specifier(self.from, path),
            None => {
                self.missing.push(module_id.to_string());
                self.paths.specifier(self.from, &format!("{module_id}.js"))
            }
        }
    }
}

impl<'a> VisitMut<'a> for Rewrite<'a, '_> {
    fn visit_call_expression(&mut self, it: &mut CallExpression<'a>) {
        if self.bind {
            self.pass_through(it);
        }
        if let Some(module_id) = self.require_argument_id(it) {
            if self.track {
                self.converted.insert(it.callee.span().start);
            }
            self.replace(it.arguments.first_mut(), &module_id);
        } else if let Some((module_id, start)) =
            self.bind.then(|| self.bound_require_id(it)).flatten()
        {
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
        if self.scoping.is_none() {
            return walk_mut::walk_program(self, it);
        }
        self.sloppy = !shadow::strict(&it.directives) && !it.source_type.is_module();
        let shadows = shadow::hoisted(&it.body, self.sloppy) || shadow::lexical(&it.body);
        self.scoped(shadows, |this| walk_mut::walk_program(this, it));
    }

    #[inline]
    fn visit_function(&mut self, it: &mut Function<'a>, flags: oxc_syntax::scope::ScopeFlags) {
        if self.scoping.is_none() {
            return walk_mut::walk_function(self, it, flags);
        }
        let sloppy = self.sloppy;
        if let Some(body) = &it.body {
            self.sloppy &= !shadow::strict(&body.directives);
        }
        let shadows = shadow::params_bind(&it.params)
            || it.body.as_ref().is_some_and(|body| {
                shadow::hoisted(&body.statements, self.sloppy) || shadow::lexical(&body.statements)
            });
        self.scoped(shadows, |this| walk_mut::walk_function(this, it, flags));
        self.sloppy = sloppy;
    }

    #[inline]
    fn visit_arrow_function_expression(&mut self, it: &mut ArrowFunctionExpression<'a>) {
        if self.scoping.is_none() {
            return walk_mut::walk_arrow_function_expression(self, it);
        }
        let body = match &it.body {
            ArrowFunctionBody::FunctionBody(body) => {
                shadow::hoisted(&body.statements, self.sloppy) || shadow::lexical(&body.statements)
            }
            _ => false,
        };
        let shadows = shadow::params_bind(&it.params) || body;
        self.scoped(shadows, |this| {
            walk_mut::walk_arrow_function_expression(this, it)
        });
    }

    #[inline]
    fn visit_block_statement(&mut self, it: &mut BlockStatement<'a>) {
        if self.scoping.is_none() {
            return walk_mut::walk_block_statement(self, it);
        }
        let shadows = shadow::lexical(&it.body);
        self.scoped(shadows, |this| walk_mut::walk_block_statement(this, it));
    }

    #[inline]
    fn visit_static_block(&mut self, it: &mut StaticBlock<'a>) {
        if self.scoping.is_none() {
            return walk_mut::walk_static_block(self, it);
        }
        let shadows = shadow::lexical(&it.body) || shadow::hoisted(&it.body, self.sloppy);
        self.scoped(shadows, |this| walk_mut::walk_static_block(this, it));
    }

    #[inline]
    fn visit_switch_statement(&mut self, it: &mut SwitchStatement<'a>) {
        if self.scoping.is_none() {
            return walk_mut::walk_switch_statement(self, it);
        }
        let shadows = it
            .cases
            .iter()
            .any(|case| shadow::lexical(&case.consequent));
        self.scoped(shadows, |this| walk_mut::walk_switch_statement(this, it));
    }

    #[inline]
    fn visit_catch_clause(&mut self, it: &mut CatchClause<'a>) {
        if self.scoping.is_none() {
            return walk_mut::walk_catch_clause(self, it);
        }
        let shadows = it
            .param
            .as_ref()
            .is_some_and(|param| shadow::binds(&param.pattern));
        self.scoped(shadows, |this| walk_mut::walk_catch_clause(this, it));
    }

    #[inline]
    fn visit_for_statement(&mut self, it: &mut ForStatement<'a>) {
        if self.scoping.is_none() {
            return walk_mut::walk_for_statement(self, it);
        }
        let head = match &it.init {
            Some(ForStatementInit::VariableDeclaration(d)) => Some(&**d),
            _ => None,
        };
        let shadows = shadow::for_head_lexical(head);
        self.scoped(shadows, |this| walk_mut::walk_for_statement(this, it));
    }

    #[inline]
    fn visit_for_in_statement(&mut self, it: &mut ForInStatement<'a>) {
        if self.scoping.is_none() {
            return walk_mut::walk_for_in_statement(self, it);
        }
        let head = match &it.left {
            ForStatementLeft::VariableDeclaration(d) => Some(&**d),
            _ => None,
        };
        let shadows = shadow::for_head_lexical(head);
        self.scoped(shadows, |this| walk_mut::walk_for_in_statement(this, it));
    }

    #[inline]
    fn visit_for_of_statement(&mut self, it: &mut ForOfStatement<'a>) {
        if self.scoping.is_none() {
            return walk_mut::walk_for_of_statement(self, it);
        }
        let head = match &it.left {
            ForStatementLeft::VariableDeclaration(d) => Some(&**d),
            _ => None,
        };
        let shadows = shadow::for_head_lexical(head);
        self.scoped(shadows, |this| walk_mut::walk_for_of_statement(this, it));
    }

    fn visit_import_declaration(&mut self, it: &mut ImportDeclaration<'a>) {
        if let Some(source) = it.source.value.as_str() {
            let specifier = self.specifier(source);
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

pub fn scoping_if_bound(program: &Program<'_>) -> Option<Scoping> {
    crate::esbuild::mentions::binds_any(program, std::iter::once(REQUIRE))
        .then(|| build_scoping(program).scoping)
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
    let paths = ModulePaths::of(bundle).matching_reference(!bind);
    let mut missing = Vec::new();
    let mut converted = Converted::new();
    for (index, (id, module)) in bundle.modules.iter_mut().enumerate() {
        let from = module.path.as_str().to_string();
        let scoping = scoping_if_bound(module.program());
        let mut rewrite = Rewrite {
            builder: AstBuilder::new(alloc),
            paths: &paths,
            from: &from,
            missing: Vec::new(),
            converted: FxHashSet::default(),
            bind,
            track: tracked.get(index).copied().unwrap_or(false),
            route: routing.and_then(|routing| {
                Some((routing.bindings.get(index)?.as_str(), &routing.available))
            }),
            scoping,
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
