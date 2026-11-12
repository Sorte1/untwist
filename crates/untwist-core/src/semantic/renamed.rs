use std::cell::RefCell;

use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_str::Ident;
use oxc_syntax::symbol::SymbolId;

struct Renamed {
    program: usize,
    symbol: SymbolId,
    from: String,
    to: String,
}

thread_local! {
    static JOURNAL: RefCell<Vec<Renamed>> = const { RefCell::new(Vec::new()) };
}

fn address(program: &Program<'_>) -> usize {
    std::ptr::from_ref(program).addr()
}

pub(crate) fn note_renames<'n>(
    program: &Program<'_>,
    scoping: &Scoping,
    renames: impl Iterator<Item = (SymbolId, &'n str)>,
) {
    let program = address(program);
    JOURNAL.with(|journal| {
        let mut journal = journal.borrow_mut();
        let valid = renames.filter(|(symbol, _)| symbol.index() < scoping.symbols_len());
        journal.extend(valid.map(|(symbol, to)| Renamed {
            program,
            symbol,
            from: scoping.symbol_name(symbol).to_string(),
            to: to.to_string(),
        }));
    });
}

pub fn renamed_since_refresh(program: &Program<'_>, name: &str) -> bool {
    let program = address(program);
    JOURNAL.with(|journal| {
        journal.borrow().iter().any(|renamed| {
            renamed.program == program && (renamed.from == name || renamed.to == name)
        })
    })
}

pub(crate) fn forget_program(program: &Program<'_>) {
    let program = address(program);
    JOURNAL.with(|journal| {
        journal
            .borrow_mut()
            .retain(|renamed| renamed.program != program);
    });
}

pub(super) fn forget() {
    JOURNAL.with(|journal| journal.borrow_mut().clear());
}

pub(super) fn replay(program: &Program<'_>, scoping: &mut Scoping) -> bool {
    let program = address(program);
    let journal = JOURNAL.with(|journal| std::mem::take(&mut *journal.borrow_mut()));
    let mut pending: Vec<&Renamed> = journal
        .iter()
        .filter(|renamed| renamed.program == program)
        .collect();
    pending.sort_by_key(|renamed| (span_start(scoping, renamed.symbol), renamed.symbol));
    let any = !pending.is_empty();
    let left: Vec<&Renamed> = pending
        .into_iter()
        .filter(|renamed| !rename(scoping, renamed))
        .collect();
    for renamed in left {
        rename(scoping, renamed);
    }
    any
}

fn span_start(scoping: &Scoping, symbol: SymbolId) -> u32 {
    if symbol.index() < scoping.symbols_len() {
        scoping.symbol_span(symbol).start
    } else {
        u32::MAX
    }
}

fn rename(scoping: &mut Scoping, renamed: &Renamed) -> bool {
    let symbol = renamed.symbol;
    if symbol.index() >= scoping.symbols_len() || scoping.symbol_name(symbol) != renamed.from {
        return false;
    }
    let scope = scoping.symbol_scope_id(symbol);
    let (from, to) = (
        Ident::from(renamed.from.as_str()),
        Ident::from(renamed.to.as_str()),
    );
    let placed = scoping.get_binding(scope, from) == Some(symbol);
    if !placed || scoping.get_binding(scope, to).is_some() {
        return false;
    }
    scoping.rename_symbol(symbol, scope, to);
    true
}
