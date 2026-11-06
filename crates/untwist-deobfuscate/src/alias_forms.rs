use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::literal::is_safe_literal;

#[derive(Hash)]
pub struct ObjectAlias {
    pub symbol: SymbolId,
    pub source: SymbolId,
    pub statement: Span,
    pub key: String,
}

fn bare_identifier<'e, 'a>(expr: &'e Expression<'a>) -> Option<&'e IdentifierReference<'a>> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => Some(id),
        _ => None,
    }
}

pub fn literal_headed_identifier<'e, 'a>(
    expr: &'e Expression<'a>,
) -> Option<&'e IdentifierReference<'a>> {
    let Expression::SequenceExpression(sequence) = expr.without_parentheses() else {
        return None;
    };
    let (last, heads) = sequence.expressions.split_last()?;
    if heads.is_empty() || !heads.iter().all(is_safe_literal) {
        return None;
    }
    bare_identifier(last)
}

fn no_parameters(params: &FormalParameters<'_>) -> bool {
    params.items.is_empty() && params.rest.is_none()
}

fn returned_by_function<'e, 'a>(function: &'e Function<'a>) -> Option<&'e IdentifierReference<'a>> {
    if function.r#async || function.generator || !no_parameters(&function.params) {
        return None;
    }
    let body = function.body.as_ref()?;
    let [Statement::ReturnStatement(ret)] = body.statements.as_slice() else {
        return None;
    };
    bare_identifier(ret.argument.as_ref()?)
}

fn returned_by_arrow<'e, 'a>(
    arrow: &'e ArrowFunctionExpression<'a>,
) -> Option<&'e IdentifierReference<'a>> {
    if arrow.r#async || !no_parameters(&arrow.params) {
        return None;
    }
    match &arrow.body {
        ArrowFunctionBody::FunctionBody(body) => {
            let [Statement::ReturnStatement(ret)] = body.statements.as_slice() else {
                return None;
            };
            bare_identifier(ret.argument.as_ref()?)
        }
        body => bare_identifier(body.as_expression()?),
    }
}

pub fn returned_identifier<'e, 'a>(
    expr: &'e Expression<'a>,
) -> Option<&'e IdentifierReference<'a>> {
    let Expression::CallExpression(call) = expr.without_parentheses() else {
        return None;
    };
    if !call.arguments.is_empty() || call.optional {
        return None;
    }
    match call.callee.without_parentheses() {
        Expression::FunctionExpression(function) => returned_by_function(function),
        Expression::ArrowFunctionExpression(arrow) => returned_by_arrow(arrow),
        _ => None,
    }
}

fn resolved(id: &IdentifierReference<'_>, scoping: &Scoping) -> Option<SymbolId> {
    scoping.get_reference(id.reference_id.get()?).symbol_id()
}

pub fn alias_source(init: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let id = match init {
        Expression::Identifier(id) => Some(&**id),
        _ => literal_headed_identifier(init).or_else(|| returned_identifier(init)),
    }?;
    resolved(id, scoping)
}

pub fn single_property_source(
    init: &Expression<'_>,
    scoping: &Scoping,
) -> Option<(SymbolId, String)> {
    let Expression::ObjectExpression(object) = init else {
        return None;
    };
    let [ObjectPropertyKind::ObjectProperty(property)] = object.properties.as_slice() else {
        return None;
    };
    if property.kind != PropertyKind::Init || property.method || property.computed {
        return None;
    }
    let key = property.key.static_name()?;
    if key == "__proto__" {
        return None;
    }
    let source = resolved(bare_identifier(&property.value)?, scoping)?;
    Some((source, key.into_owned()))
}

struct MemberReads<'s> {
    scoping: &'s Scoping,
    wanted: &'s FxHashMap<SymbolId, &'s str>,
    reads: FxHashMap<SymbolId, Vec<Span>>,
    spoiled: FxHashSet<SymbolId>,
}

impl MemberReads<'_> {
    fn object_symbol(&self, member: &StaticMemberExpression<'_>) -> Option<SymbolId> {
        let id = bare_identifier(&member.object)?;
        resolved(id, self.scoping).filter(|symbol| self.wanted.contains_key(symbol))
    }
}

impl<'a> VisitJs<'a> for MemberReads<'_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if let Expression::StaticMemberExpression(member) = it {
            if let Some(symbol) = self.object_symbol(member) {
                let matches = self.wanted.get(&symbol) == Some(&member.property.name.as_str());
                if matches && !member.optional {
                    self.reads.entry(symbol).or_default().push(member.span);
                } else {
                    self.spoiled.insert(symbol);
                }
            }
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            if let Expression::StaticMemberExpression(member) = it.argument.without_parentheses() {
                if let Some(symbol) = self.object_symbol(member) {
                    self.spoiled.insert(symbol);
                }
            }
        }
        walk_js::walk_unary_expression(self, it);
    }
}

pub fn member_reads(
    program: &Program<'_>,
    scoping: &Scoping,
    wanted: &FxHashMap<SymbolId, &str>,
    checked: bool,
) -> FxHashMap<SymbolId, Vec<Span>> {
    let mut walk = MemberReads {
        scoping,
        wanted,
        reads: FxHashMap::default(),
        spoiled: FxHashSet::default(),
    };
    walk.visit_program(program);
    let MemberReads {
        mut reads, spoiled, ..
    } = walk;
    wanted
        .keys()
        .filter_map(|symbol| {
            let spans = reads.remove(symbol).unwrap_or_default();
            let whole = scoping.get_resolved_reference_ids(*symbol).len() == spans.len();
            (!checked || (whole && !spoiled.contains(symbol))).then_some((*symbol, spans))
        })
        .collect()
}

#[derive(Default)]
struct ThisScan {
    found: bool,
}

impl<'a> VisitJs<'a> for ThisScan {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {}
    fn visit_class(&mut self, _it: &Class<'a>) {}
    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.found = true;
    }
}

fn ignores_this(function: &Function<'_>) -> bool {
    let mut scan = ThisScan::default();
    scan.visit_formal_parameters(&function.params);
    if let Some(body) = &function.body {
        scan.visit_function_body(body);
    }
    !scan.found
}

struct DecoderBodies<'d> {
    decoders: &'d FxHashSet<SymbolId>,
    seen: usize,
    reads_this: bool,
}

impl<'a> VisitJs<'a> for DecoderBodies<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let symbol = it.id.as_ref().and_then(|id| id.symbol_id.get());
        if symbol.is_some_and(|symbol| self.decoders.contains(&symbol)) {
            self.seen += 1;
            self.reads_this |= !ignores_this(it);
        }
        walk_js::walk_function(self, it, flags);
    }
}

pub fn decoders_ignore_this(program: &Program<'_>, decoders: &FxHashSet<SymbolId>) -> bool {
    let mut scan = DecoderBodies {
        decoders,
        seen: 0,
        reads_this: false,
    };
    scan.visit_program(program);
    !scan.reads_this && scan.seen == decoders.len()
}

pub fn deleted_span(it: &Expression<'_>) -> Option<Span> {
    let Expression::UnaryExpression(unary) = it else {
        return None;
    };
    (unary.operator == UnaryOperator::Delete).then(|| unary.argument.without_parentheses().span())
}
