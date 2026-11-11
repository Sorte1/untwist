use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::edit::ExprEdits;
use untwist_core::ast_util::inline::returned_expression;
use untwist_core::ast_util::matcher::call_parts;
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::pass::Cx;

use crate::mutants::wrapper_constants_trusted;
use crate::stamp::{decoder_index, is_machinery_name};

fn primitive(expr: &Expression<'_>) -> bool {
    match expr.without_parentheses() {
        Expression::StringLiteral(_) | Expression::NumericLiteral(_) => true,
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::UnaryNegation
                && matches!(unary.argument, Expression::NumericLiteral(_))
        }
        _ => false,
    }
}

struct Constant<'p> {
    init: &'p Expression<'p>,
    position: u32,
    function: Option<ScopeId>,
}

struct Wrapper {
    symbol: SymbolId,
    function: Option<ScopeId>,
    reads: Vec<(Span, SymbolId)>,
}

struct Reads<'s> {
    scoping: &'s Scoping,
    params: Vec<SymbolId>,
    found: Vec<(Span, SymbolId)>,
}

impl<'a> VisitJs<'a> for Reads<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if is_machinery_name(it.name.as_str()) {
            return;
        }
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        if let Some(symbol) = symbol.filter(|symbol| !self.params.contains(symbol)) {
            self.found.push((it.span, symbol));
        }
    }
}

struct Search<'s, 'p> {
    scoping: &'s Scoping,
    constants: FxHashMap<SymbolId, Constant<'p>>,
    wrappers: Vec<Wrapper>,
    functions: Vec<ScopeId>,
    call_reads: Vec<CallRead>,
    position: u32,
}

struct CallRead {
    callee: Option<SymbolId>,
    decoder: bool,
    span: Span,
    symbol: SymbolId,
    function: Option<ScopeId>,
    position: u32,
}

fn binds_at_body_level(scoping: &Scoping, symbol: SymbolId) -> bool {
    let scope = scoping.symbol_scope_id(symbol);
    scope == scoping.root_scope_id() || scoping.scope_flags(scope).is_function()
}

impl<'s, 'p> Search<'s, 'p> {
    fn advance(&mut self) -> u32 {
        self.position += 1;
        self.position
    }

    fn wrapper(&self, function: &Function<'p>) -> Option<Wrapper> {
        let symbol = function.id.as_ref()?.symbol_id.get()?;
        let (callee, arguments) = call_parts(returned_expression(function)?)?;
        identifier_name(callee).and_then(decoder_index)?;
        let mut params = Vec::new();
        for item in &function.params.items {
            let BindingPattern::BindingIdentifier(id) = &item.pattern else {
                return None;
            };
            params.push(id.symbol_id.get()?);
        }
        let mut reads = Reads {
            scoping: self.scoping,
            params,
            found: Vec::new(),
        };
        for argument in arguments {
            reads.visit_argument(argument);
        }
        (!reads.found.is_empty()).then(|| Wrapper {
            symbol,
            function: self.functions.last().copied(),
            reads: reads.found,
        })
    }
}

impl<'s, 'p> VisitJs<'p> for Search<'s, 'p> {
    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'p>) {
        if it.kind == VariableDeclarationKind::Const {
            let declaration: &'p VariableDeclaration<'p> = self.alloc(it);
            for declarator in &declaration.declarations {
                let (BindingPattern::BindingIdentifier(id), Some(init)) =
                    (&declarator.id, declarator.init.as_ref())
                else {
                    continue;
                };
                let Some(symbol) = id.symbol_id.get() else {
                    continue;
                };
                if primitive(init)
                    && (wrapper_constants_trusted() || binds_at_body_level(self.scoping, symbol))
                {
                    let position = self.advance();
                    self.constants.insert(
                        symbol,
                        Constant {
                            init,
                            position,
                            function: self.functions.last().copied(),
                        },
                    );
                }
            }
        }
        walk_js::walk_variable_declaration(self, it);
    }

    fn visit_function(&mut self, it: &Function<'p>, flags: ScopeFlags) {
        if it.is_declaration() {
            let function: &'p Function<'p> = self.alloc(it);
            if let Some(wrapper) = self.wrapper(function) {
                self.wrappers.push(wrapper);
            }
        }
        let Some(scope) = it.scope_id.get() else {
            return;
        };
        self.functions.push(scope);
        walk_js::walk_function(self, it, flags);
        self.functions.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'p>) {
        let Some(scope) = it.scope_id.get() else {
            return;
        };
        self.functions.push(scope);
        walk_js::walk_arrow_function_expression(self, it);
        self.functions.pop();
    }

    fn visit_sequence_expression(&mut self, it: &SequenceExpression<'p>) {
        if let Some((last, leading)) = it.expressions.split_last() {
            let decodes = call_parts(last.without_parentheses())
                .and_then(|(callee, _)| identifier_name(callee))
                .and_then(decoder_index)
                .is_some();
            if decodes {
                for expr in leading {
                    let Expression::AssignmentExpression(assignment) = expr.without_parentheses()
                    else {
                        continue;
                    };
                    let Expression::Identifier(read) = assignment.right.without_parentheses()
                    else {
                        continue;
                    };
                    let symbol = read
                        .reference_id
                        .get()
                        .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
                    if let Some(symbol) = symbol {
                        let position = self.advance();
                        self.call_reads.push(CallRead {
                            callee: None,
                            decoder: true,
                            span: read.span,
                            symbol,
                            function: self.functions.last().copied(),
                            position,
                        });
                    }
                }
            }
        }
        walk_js::walk_sequence_expression(self, it);
    }
}

struct Uses<'s> {
    scoping: &'s Scoping,
    wanted: Vec<SymbolId>,
    constants: FxHashSet<SymbolId>,
    functions: Vec<ScopeId>,
    position: u32,
    unscoped: bool,
    declared_at: FxHashMap<SymbolId, u32>,
    found: FxHashMap<SymbolId, Vec<(u32, Option<ScopeId>)>>,
}

impl<'a> VisitJs<'a> for Uses<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let Some(scope) = it.scope_id.get() else {
            self.unscoped = true;
            return;
        };
        self.functions.push(scope);
        walk_js::walk_function(self, it, flags);
        self.functions.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let Some(scope) = it.scope_id.get() else {
            self.unscoped = true;
            return;
        };
        self.functions.push(scope);
        walk_js::walk_arrow_function_expression(self, it);
        self.functions.pop();
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        walk_js::walk_variable_declarator(self, it);
        if let BindingPattern::BindingIdentifier(id) = &it.id {
            if let Some(symbol) = id.symbol_id.get().filter(|s| self.constants.contains(s)) {
                self.position += 1;
                self.declared_at.insert(symbol, self.position);
            }
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        if let Some(symbol) = symbol.filter(|symbol| self.wanted.contains(symbol)) {
            self.position += 1;
            self.found
                .entry(symbol)
                .or_default()
                .push((self.position, self.functions.last().copied()));
        }
    }
}

fn settled(scoping: &Scoping, symbol: SymbolId) -> bool {
    scoping
        .symbol_flags(symbol)
        .contains(SymbolFlags::ConstVariable)
        && scoping
            .get_resolved_references(symbol)
            .all(|reference| !reference.is_write())
}

fn runs_after(uses: &Uses<'_>, wrapper: &Wrapper, constant: SymbolId) -> bool {
    if wrapper_constants_trusted() {
        return true;
    }
    if uses.unscoped {
        return false;
    }
    let Some(declared) = uses.declared_at.get(&constant) else {
        return false;
    };
    uses.found.get(&wrapper.symbol).is_some_and(|references| {
        references
            .iter()
            .all(|(position, function)| position > declared && *function == wrapper.function)
    })
}

pub(crate) fn inline_wrapper_constants<'a>(
    program: &mut Program<'a>,
    cx: &mut Cx<'a, '_>,
) -> usize {
    let edits = {
        let view: &Program<'a> = program;
        let mut search = Search {
            scoping: cx.scoping(),
            constants: FxHashMap::default(),
            wrappers: Vec::new(),
            functions: Vec::new(),
            call_reads: Vec::new(),
            position: 0,
        };
        search.visit_program(view);
        let candidates: Vec<SymbolId> = search
            .wrappers
            .iter()
            .filter(|wrapper| {
                wrapper
                    .reads
                    .iter()
                    .all(|(_, symbol)| search.constants.contains_key(symbol))
            })
            .map(|wrapper| wrapper.symbol)
            .collect();
        let mut uses = Uses {
            scoping: cx.scoping(),
            constants: search.constants.keys().copied().collect(),
            wanted: candidates,
            functions: Vec::new(),
            position: 0,
            unscoped: false,
            declared_at: FxHashMap::default(),
            found: FxHashMap::default(),
        };
        if !uses.wanted.is_empty() {
            uses.visit_program(view);
        }
        let mut edits = ExprEdits::new();
        for wrapper in &search.wrappers {
            let scope = cx.scoping().symbol_scope_id(wrapper.symbol);
            let resolved: Option<Vec<(Span, &Constant<'_>)>> = wrapper
                .reads
                .iter()
                .map(|(span, symbol)| {
                    let constant = search.constants.get(symbol)?;
                    let same_scope = cx.scoping().symbol_scope_id(*symbol) == scope;
                    (same_scope
                        && settled(cx.scoping(), *symbol)
                        && runs_after(&uses, wrapper, *symbol))
                    .then_some((*span, constant))
                })
                .collect();
            let Some(resolved) = resolved else {
                continue;
            };
            for (span, constant) in resolved {
                let value = constant.init.clone_in(cx.allocator());
                edits.insert(span, value);
            }
        }
        let wrappers: Vec<SymbolId> = search.wrappers.iter().map(|w| w.symbol).collect();
        for read in &search.call_reads {
            let wanted =
                read.decoder || read.callee.is_some_and(|callee| wrappers.contains(&callee));
            let Some(constant) = search.constants.get(&read.symbol) else {
                continue;
            };
            let ordered = wrapper_constants_trusted()
                || (read.position > constant.position && read.function == constant.function);
            if wanted && ordered && settled(cx.scoping(), read.symbol) {
                edits.insert(read.span, constant.init.clone_in(cx.allocator()));
            }
        }
        edits
    };
    if edits.is_empty() {
        return 0;
    }
    edits.apply(program, cx.allocator(), cx.scoping_mut())
}
