use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::prop::number_key;

use crate::matcher::argument_expression;

const REQUIRE: &str = "require";
const PLAIN_HELPERS: [&str; 5] = ["d", "e", "o", "p", "r"];
const LOADS: u32 = 1;
const AS_IS: u32 = 8;
const MAX_MODE: f64 = 255.0;
const PROTOTYPE_NAMES: [&str; 12] = [
    "constructor",
    "toString",
    "toLocaleString",
    "valueOf",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "__proto__",
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
];

#[derive(Default)]
pub struct Facts {
    pub targets: Vec<String>,
    pub refused: Vec<String>,
    pub global: bool,
    pub leftover: bool,
    pub wants_scoping: bool,
}

pub struct Scan<'s> {
    scoping: Option<&'s Scoping>,
    candidates: &'s FxHashSet<String>,
    inert: &'s FxHashSet<String>,
    dropped: &'s FxHashSet<usize>,
    depth: u32,
    returns: u32,
    suppress: bool,
    safe: usize,
    unsafe_object: usize,
    require_callee: usize,
    tracked: FxHashMap<SymbolId, String>,
    safe_reads: Vec<u32>,
    interop_args: Vec<SymbolId>,
    import_at: usize,
    import_param: Option<SymbolId>,
    facts: Facts,
}

fn address<T>(it: &T) -> usize {
    std::ptr::from_ref(it) as usize
}

pub fn literal_id(argument: Option<&Argument<'_>>) -> Option<String> {
    match argument.and_then(argument_expression)? {
        Expression::NumericLiteral(literal) => Some(number_key(literal.value)),
        Expression::StringLiteral(literal) if !literal.lone_surrogates => {
            Some(literal.value.to_string())
        }
        _ => None,
    }
}

fn mode(argument: Option<&Argument<'_>>) -> Option<u32> {
    match argument.and_then(argument_expression)? {
        Expression::NumericLiteral(literal)
            if literal.value.fract() == 0.0 && (0.0..=MAX_MODE).contains(&literal.value) =>
        {
            Some(literal.value as u32)
        }
        _ => None,
    }
}

fn is_require(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::Identifier(id) if id.name == REQUIRE)
}

fn require_member<'k>(expr: &'k Expression<'_>) -> Option<&'k str> {
    match expr {
        Expression::StaticMemberExpression(member) if is_require(&member.object) => {
            Some(member.property.name.as_str())
        }
        _ => None,
    }
}

fn runtime_callee<'k>(callee: &'k Expression<'_>) -> Option<(&'k str, usize)> {
    if let Some(helper) = require_member(callee) {
        return (!PLAIN_HELPERS.contains(&helper)).then_some((helper, 0));
    }
    let Expression::StaticMemberExpression(member) = callee else {
        return None;
    };
    let helper = require_member(&member.object)?;
    (member.property.name == "bind").then_some((helper, 1))
}

fn read_object<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k Expression<'a>> {
    let (object, name) = match expr {
        Expression::StaticMemberExpression(member) => {
            (&member.object, Some(member.property.name.as_str()))
        }
        Expression::ComputedMemberExpression(member) => match &member.expression {
            Expression::StringLiteral(key) => (&member.object, Some(key.value.as_str())),
            Expression::NumericLiteral(_) => (&member.object, None),
            _ => return None,
        },
        _ => return None,
    };
    let inherited = name.is_some_and(|name| PROTOTYPE_NAMES.contains(&name));
    (!inherited).then_some(object)
}

fn object_address(object: &Expression<'_>) -> usize {
    match object {
        Expression::Identifier(id) => address(&**id),
        Expression::CallExpression(call) => address(&**call),
        _ => 0,
    }
}

fn loading_call<'k, 'a>(expr: &'k Expression<'a>) -> Option<(&'k CallExpression<'a>, String)> {
    let Expression::CallExpression(call) = expr else {
        return None;
    };
    let loads = || mode(call.arguments.get(1)).is_some_and(|m| m & AS_IS != 0 && m & LOADS != 0);
    if !is_require(&call.callee) && !(require_member(&call.callee) == Some("t") && loads()) {
        return None;
    }
    Some((call, literal_id(call.arguments.first())?))
}

fn member_call<'k, 'a>(
    expr: &'k Expression<'a>,
    name: &str,
) -> Option<(&'k Expression<'a>, &'k [Argument<'a>])> {
    let Expression::CallExpression(call) = expr else {
        return None;
    };
    match &call.callee {
        Expression::StaticMemberExpression(member) if member.property.name == name => {
            Some((&member.object, &call.arguments))
        }
        _ => None,
    }
}

fn chunk_load(expr: &Expression<'_>) -> bool {
    member_call(expr, "e").is_some_and(|(object, arguments)| {
        is_require(object) && arguments.len() == 1 && literal_id(arguments.first()).is_some()
    })
}

fn chunk_loads(arguments: &[Argument<'_>]) -> bool {
    let [Argument::ArrayExpression(list)] = arguments else {
        return false;
    };
    list.elements
        .iter()
        .all(|element| element.as_expression().is_some_and(chunk_load))
}

fn promise_source<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k IdentifierReference<'a>> {
    let (object, _) = member_call(expr, "resolve")
        .filter(|(_, arguments)| arguments.is_empty())
        .or_else(|| member_call(expr, "all").filter(|(_, arguments)| chunk_loads(arguments)))?;
    match object {
        Expression::Identifier(promise) if promise.name == "Promise" => Some(promise),
        _ => None,
    }
}

fn sole_param<'k, 'a>(argument: Option<&'k Argument<'a>>) -> Option<&'k BindingIdentifier<'a>> {
    let Some(Argument::ArrowFunctionExpression(arrow)) = argument else {
        return None;
    };
    let params = &arrow.params;
    match (&params.items[..], &params.rest) {
        ([item], None) if item.initializer.is_none() => match &item.pattern {
            BindingPattern::BindingIdentifier(id) => Some(id),
            _ => None,
        },
        _ => None,
    }
}

struct DynamicImport<'k, 'a> {
    bind: &'k CallExpression<'a>,
    param: &'k BindingIdentifier<'a>,
    promise: Option<&'k IdentifierReference<'a>>,
}

fn dynamic_import<'k, 'a>(outer: &'k CallExpression<'a>) -> Option<DynamicImport<'k, 'a>> {
    let (inner, outer_arguments) = match &outer.callee {
        Expression::StaticMemberExpression(member) if member.property.name == "then" => {
            (&member.object, &outer.arguments)
        }
        _ => return None,
    };
    if outer_arguments.len() > 2 {
        return None;
    }
    let param = sole_param(outer_arguments.first())?;
    let (source, inner_arguments) = member_call(inner, "then")?;
    let [Argument::CallExpression(bind)] = inner_arguments else {
        return None;
    };
    let bound = member_call_of(bind, "bind")?;
    let this = bind.arguments.first().and_then(argument_expression);
    let bound_this =
        this.is_some_and(|this| is_require(this) || matches!(this, Expression::NullLiteral(_)));
    if !is_require(bound) || bind.arguments.len() != 2 || !bound_this {
        return None;
    }
    literal_id(bind.arguments.get(1))?;
    let promise = match chunk_load(source) {
        true => None,
        false => Some(promise_source(source)?),
    };
    Some(DynamicImport {
        bind,
        param,
        promise,
    })
}

fn member_call_of<'k, 'a>(call: &'k CallExpression<'a>, name: &str) -> Option<&'k Expression<'a>> {
    match &call.callee {
        Expression::StaticMemberExpression(member) if member.property.name == name => {
            Some(&member.object)
        }
        _ => None,
    }
}

impl<'s> Scan<'s> {
    pub fn new(
        scoping: Option<&'s Scoping>,
        (candidates, inert): (&'s FxHashSet<String>, &'s FxHashSet<String>),
        dropped: &'s FxHashSet<usize>,
    ) -> Self {
        Scan {
            scoping,
            candidates,
            inert,
            dropped,
            depth: 0,
            returns: 0,
            suppress: false,
            safe: 0,
            unsafe_object: 0,
            require_callee: 0,
            tracked: FxHashMap::default(),
            safe_reads: vec![0; scoping.map_or(0, Scoping::symbols_len)],
            interop_args: Vec::new(),
            import_at: 0,
            import_param: None,
            facts: Facts::default(),
        }
    }

    pub fn run(mut self, program: &Program<'_>) -> Facts {
        for (index, statement) in program.body.iter().enumerate() {
            self.suppress = self.dropped.contains(&index);
            self.visit_statement(statement);
        }
        self.suppress = false;
        self.finish()
    }

    fn finish(mut self) -> Facts {
        let Some(scoping) = self.scoping else {
            return self.facts;
        };
        for (symbol, id) in &self.tracked {
            let reads = self.safe_reads[symbol.index()] as usize;
            let escaped = scoping.get_resolved_reference_ids(*symbol).len() != reads
                || !scoping.symbol_redeclarations(*symbol).is_empty();
            if escaped {
                self.facts.refused.push(id.clone());
            }
        }
        let attributed = self.interop_args.iter();
        self.facts.global |= attributed
            .into_iter()
            .any(|s| !self.tracked.contains_key(s));
        self.facts
    }

    fn symbol(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        self.scoping?.get_reference(id.reference_id()).symbol_id()
    }

    fn interop(&mut self, argument: Option<&Argument<'_>>) {
        match argument.and_then(argument_expression) {
            Some(Expression::Identifier(id)) => match (self.scoping, self.symbol(id)) {
                (None, _) => self.facts.wants_scoping = true,
                (Some(_), Some(symbol)) => self.interop_args.push(symbol),
                (Some(_), None) => self.facts.global = true,
            },
            Some(Expression::CallExpression(call)) if is_require(&call.callee) => {}
            _ => self.facts.global = true,
        }
    }

    fn refuse(&mut self, id: Option<String>) {
        match id {
            Some(id) => {
                self.facts.targets.push(id.clone());
                self.facts.refused.push(id);
            }
            None => self.facts.global = true,
        }
    }

    fn helper_call(&mut self, it: &CallExpression<'_>, consumed: bool) -> Option<bool> {
        let (helper, skip) = runtime_callee(&it.callee)?;
        let this = it.arguments.first().and_then(argument_expression);
        let runtime_this = this.is_some_and(is_require);
        let bound_this = runtime_this || matches!(this, Some(Expression::NullLiteral(_)));
        let bound = skip == 1 || helper == "bind";
        if skip == 1 && !bound_this {
            self.facts.global = true;
        }
        let id = literal_id(it.arguments.get(skip));
        let mode = mode(it.arguments.get(skip + 1));
        let as_is = mode.is_some_and(|m| m & AS_IS != 0);
        let loads = mode.is_some_and(|m| m & LOADS != 0);
        match (helper, skip) {
            ("n", 0) => self.interop(it.arguments.first()),
            ("t", 0) if as_is && (!loads || consumed) => match id {
                Some(id) => self.facts.targets.push(id),
                None => self.facts.global = true,
            },
            ("t", 1) if as_is && !loads => match id {
                Some(id) => self.facts.targets.push(id),
                None => self.facts.global = true,
            },
            ("t", _) => self.refuse(id),
            ("bind", 0) => self.bound_require(it),
            _ => self.facts.global = true,
        }
        Some(bound && runtime_this)
    }

    fn bound_require(&mut self, it: &CallExpression<'_>) {
        let id = literal_id(it.arguments.get(1));
        let imported = self.import_at == address(it);
        match (id, imported.then_some(self.import_param).flatten()) {
            (Some(id), Some(param)) => {
                self.tracked.insert(param, id.clone());
                self.facts.targets.push(id);
            }
            (Some(id), None) if imported && self.scoping.is_none() => {
                self.facts.wants_scoping |= self.candidates.contains(&id);
                self.refuse(Some(id));
            }
            (id, _) => self.refuse(id),
        }
    }

    fn note_dynamic_import(&mut self, it: &CallExpression<'_>) {
        let Some(found) = dynamic_import(it) else {
            return;
        };
        self.import_at = address(found.bind);
        self.import_param = None;
        let Some(scoping) = self.scoping else {
            return;
        };
        let global = found.promise.is_none_or(|promise| {
            scoping
                .get_reference(promise.reference_id())
                .symbol_id()
                .is_none()
        });
        if global {
            self.import_param = Some(found.param.symbol_id());
        }
    }

    fn require_call(&mut self, it: &CallExpression<'_>, consumed: bool) {
        match literal_id(it.arguments.first()) {
            Some(id) if consumed => self.facts.targets.push(id),
            id => self.refuse(id),
        }
        if let Expression::Identifier(callee) = &it.callee {
            self.require_callee = address(&**callee);
        }
    }

    fn top_level(&mut self) {
        self.facts.leftover |= self.depth == 0 && !self.suppress;
    }
}

impl<'a> VisitJs<'a> for Scan<'_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if let Some(object) = read_object(it) {
            let object = object_address(object);
            if object != 0 && object != self.unsafe_object {
                self.safe = object;
            }
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let consumed = self.safe == address(it);
        self.note_dynamic_import(it);
        if let Some(object) = read_object(it.callee.without_parentheses()) {
            self.unsafe_object = object_address(object);
        }
        if is_require(&it.callee) {
            self.require_call(it, consumed);
        } else if let Some(skip_this) = self.helper_call(it, consumed) {
            let skipped = usize::from(skip_this);
            for argument in it.arguments.iter().skip(skipped) {
                self.visit_argument(argument);
            }
            return;
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        if let Some(object) = read_object(it.tag.without_parentheses()) {
            self.unsafe_object = object_address(object);
        }
        walk_js::walk_tagged_template_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            if let Some(object) = read_object(it.argument.without_parentheses()) {
                self.unsafe_object = object_address(object);
            }
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        let loading = it.init.as_ref().and_then(loading_call);
        if let (Some((call, id)), BindingPattern::BindingIdentifier(binding)) = (loading, &it.id) {
            self.safe = address(call);
            match self.scoping {
                Some(_) => {
                    self.tracked.insert(binding.symbol_id(), id);
                }
                None => self.facts.wants_scoping |= self.candidates.contains(&id),
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }

    fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
        if let Expression::CallExpression(call) = &it.expression {
            self.safe = address(&**call);
        }
        walk_js::walk_expression_statement(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        let name = it.property.name.as_str();
        if is_require(&it.object) {
            self.facts.global |= !PLAIN_HELPERS.contains(&name) && !self.inert.contains(name);
            return;
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_identifier_name(&mut self, it: &IdentifierName<'a>) {
        self.facts.global |= it.name == "__esModule";
    }

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        self.facts.global |= it.value == "__esModule";
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let here = address(it);
        match it.name.as_str() {
            "exports" | "module" | "eval" => self.facts.leftover |= !self.suppress,
            "arguments" => self.top_level(),
            "await" | "yield" => self.facts.leftover = true,
            REQUIRE => self.facts.global |= here != self.require_callee,
            _ => {}
        }
        if self.safe == here {
            if let Some(symbol) = self.symbol(it) {
                self.safe_reads[symbol.index()] += 1;
            }
        }
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.facts.leftover |= it.name == "await" || it.name == "yield";
    }

    fn visit_label_identifier(&mut self, it: &LabelIdentifier<'a>) {
        self.facts.leftover |= it.name == "await" || it.name == "yield";
    }

    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.top_level();
    }

    fn visit_new_target(&mut self, _it: &NewTarget) {
        self.top_level();
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        self.facts.leftover |= self.returns == 0 && !self.suppress;
        walk_js::walk_return_statement(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.depth += 1;
        self.returns += 1;
        walk_js::walk_function(self, it, flags);
        self.returns -= 1;
        self.depth -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.returns += 1;
        walk_js::walk_arrow_function_expression(self, it);
        self.returns -= 1;
    }

    fn visit_property_definition(&mut self, it: &PropertyDefinition<'a>) {
        self.visit_decorators(&it.decorators);
        self.visit_property_key(&it.key);
        self.depth += 1;
        if let Some(value) = &it.value {
            self.visit_expression(value);
        }
        self.depth -= 1;
    }

    fn visit_accessor_property(&mut self, it: &AccessorProperty<'a>) {
        self.visit_decorators(&it.decorators);
        self.visit_property_key(&it.key);
        self.depth += 1;
        if let Some(value) = &it.value {
            self.visit_expression(value);
        }
        self.depth -= 1;
    }

    fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
        self.depth += 1;
        walk_js::walk_static_block(self, it);
        self.depth -= 1;
    }
}
