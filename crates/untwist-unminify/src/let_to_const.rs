use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::{Span, SPAN};
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashSet;
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::{
    assumed_hazard, effectively_constant_bindings, has_module_syntax, scope_hazard,
};

pub const PASS: &str = "let-to-const";

pub use crate::notes::LET_TO_CONST_SCOPE_HAZARD as SCOPE_HAZARD;

#[derive(Default)]
pub struct LetToConst {
    constant: FxHashSet<SymbolId>,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(LetToConst::default())
}

fn declarator_symbol(declarator: &VariableDeclarator<'_>) -> Option<SymbolId> {
    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
        return None;
    };
    declarator.init.is_some().then(|| id.symbol_id.get())?
}

fn is_let_symbol(flags: SymbolFlags) -> bool {
    flags.contains(SymbolFlags::BlockScopedVariable)
        && !flags.intersects(SymbolFlags::ConstVariable | SymbolFlags::CatchVariable)
}

fn global_lexical_scope(program: &Program<'_>, scoping: &Scoping) -> Option<ScopeId> {
    (!has_module_syntax(program)).then(|| scoping.root_scope_id())
}

fn let_symbols(program: &Program<'_>, scoping: &Scoping) -> Vec<SymbolId> {
    let global_lexical = global_lexical_scope(program, scoping);
    scoping
        .symbol_ids()
        .filter(|symbol| is_let_symbol(scoping.symbol_flags(*symbol)))
        .filter(|symbol| Some(scoping.symbol_scope_id(*symbol)) != global_lexical)
        .collect()
}

fn is_let(declaration: &VariableDeclaration<'_>) -> bool {
    declaration.kind == VariableDeclarationKind::Let && !declaration.declare
}

fn declarator_is_constant(
    declarator: &VariableDeclarator<'_>,
    constant: &FxHashSet<SymbolId>,
) -> bool {
    declarator_symbol(declarator).is_some_and(|symbol| constant.contains(&symbol))
}

fn split_points(
    declaration: &VariableDeclaration<'_>,
    constant: &FxHashSet<SymbolId>,
) -> Option<Vec<bool>> {
    let kinds: Vec<bool> = declaration
        .declarations
        .iter()
        .map(|declarator| declarator_is_constant(declarator, constant))
        .collect();
    kinds
        .iter()
        .any(|is_constant| *is_constant)
        .then_some(kinds)
}

fn kind_of(is_constant: bool) -> VariableDeclarationKind {
    if is_constant {
        VariableDeclarationKind::Const
    } else {
        VariableDeclarationKind::Let
    }
}

fn declaration_of<'a>(
    kind: VariableDeclarationKind,
    declarators: Vec<VariableDeclarator<'a>>,
    span: Span,
    cx: &Cx<'a, '_>,
) -> Statement<'a> {
    let declarators = AVec::from_iter_in(declarators, cx);
    Statement::new_variable_declaration(span, kind, declarators, false, cx)
}

fn rebuilt_runs<'a>(
    declaration: VariableDeclaration<'a>,
    kinds: &[bool],
    cx: &Cx<'a, '_>,
) -> Vec<Statement<'a>> {
    let span = declaration.span;
    let mut out = Vec::new();
    let mut run: Vec<VariableDeclarator<'a>> = Vec::new();
    let mut run_is_constant = kinds.first().copied().unwrap_or(false);
    for (declarator, is_constant) in declaration.declarations.into_iter().zip(kinds) {
        if *is_constant != run_is_constant && !run.is_empty() {
            let kind = kind_of(run_is_constant);
            out.push(declaration_of(kind, std::mem::take(&mut run), span, cx));
        }
        run_is_constant = *is_constant;
        run.push(declarator);
    }
    out.push(declaration_of(kind_of(run_is_constant), run, span, cx));
    out
}

fn mark_const(stmts: &[Statement<'_>], constant: &FxHashSet<SymbolId>, scoping: &mut Scoping) {
    let declarations = stmts.iter().filter_map(|statement| match statement {
        Statement::VariableDeclaration(declaration)
            if declaration.kind == VariableDeclarationKind::Const =>
        {
            Some(declaration)
        }
        _ => None,
    });
    for declarator in declarations.flat_map(|declaration| declaration.declarations.iter()) {
        if let Some(symbol) = declarator_symbol(declarator).filter(|s| constant.contains(s)) {
            *scoping.symbol_flags_mut(symbol) |= SymbolFlags::ConstVariable;
        }
    }
}

fn rebuild<'a>(
    stmts: &mut AVec<'a, Statement<'a>>,
    constant: &FxHashSet<SymbolId>,
    cx: &Cx<'a, '_>,
) -> usize {
    let mut index = 0;
    let mut changed = 0;
    while index < stmts.len() {
        let kinds = match stmts.get(index) {
            Some(Statement::VariableDeclaration(declaration)) if is_let(declaration) => {
                split_points(declaration, constant)
            }
            _ => None,
        };
        let Some(kinds) = kinds else {
            index += 1;
            continue;
        };
        let Some(slot) = stmts.get_mut(index) else {
            break;
        };
        let placeholder = Statement::new_empty_statement(SPAN, cx);
        let declaration = match std::mem::replace(slot, placeholder) {
            Statement::VariableDeclaration(declaration) => declaration,
            other => {
                *slot = other;
                index += 1;
                continue;
            }
        };
        let runs = rebuilt_runs(declaration.unbox(), &kinds, cx);
        let count = runs.len();
        stmts.splice(index..=index, runs);
        index += count;
        changed += 1;
    }
    changed
}

impl<'a> Pass<'a> for LetToConst {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM | Interest::EXIT_STATEMENTS
    }

    fn requires_extended(&self) -> bool {
        true
    }

    fn reset(&mut self) {
        self.constant.clear();
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.constant.clear();
        let scoping = cx.scoping();
        if scope_hazard(scoping)
            .or_else(|| assumed_hazard(scoping))
            .is_some()
        {
            cx.note(SCOPE_HAZARD, None);
            return;
        }
        let candidates = let_symbols(program, cx.scoping());
        self.constant = effectively_constant_bindings(cx.scoping(), &candidates);
    }

    fn exit_statements(&mut self, stmts: &mut AVec<'a, Statement<'a>>, cx: &mut Cx<'a, '_>) {
        if self.constant.is_empty() {
            return;
        }
        let rebuilt = rebuild(stmts, &self.constant, cx);
        if rebuilt > 0 {
            mark_const(stmts, &self.constant, cx.scoping_mut());
        }
        cx.changed_by(rebuilt);
    }
}
