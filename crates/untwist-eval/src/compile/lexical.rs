use std::rc::Rc;

use oxc_ast::ast::{BindingPattern, Statement, VariableDeclarationKind};

use crate::compile::scope::{block_layout, LayoutBuilder, Provable, ScopeChain};
use crate::compile::stmt::lazies;
use crate::compile::Lazy;
use crate::frame::{FrameLayout, SlotKind};
use crate::names::Name;
use crate::names::Names;

pub(crate) fn slot_kind(kind: VariableDeclarationKind) -> SlotKind {
    match kind {
        VariableDeclarationKind::Const => SlotKind::Constant,
        _ => SlotKind::Mutable,
    }
}

pub(crate) fn is_lexical(kind: VariableDeclarationKind) -> bool {
    matches!(
        kind,
        VariableDeclarationKind::Let | VariableDeclarationKind::Const
    )
}

pub(crate) fn scan<'a>(
    names: &mut Names<'a>,
    builder: &mut LayoutBuilder,
    statements: &'a [Statement<'a>],
) -> Option<Box<[(usize, u32)]>> {
    let mut declared = Vec::new();
    for (index, statement) in statements.iter().enumerate() {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        if !is_lexical(declaration.kind) {
            continue;
        }
        let kind = slot_kind(declaration.kind);
        for declarator in &declaration.declarations {
            if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                let name = names.intern(id.name.as_str());
                declared.push((index, builder.slot_of_kind(name, kind)?));
            }
        }
    }
    Some(declared.into_boxed_slice())
}

const FIRST: u32 = 1;

fn declaration_order(len: usize, declared: &[(usize, u32)]) -> Option<Rc<[u32]>> {
    let mut order: Rc<[u32]> = std::iter::repeat_n(0, len).collect();
    let slots = Rc::get_mut(&mut order)?;
    for (at, slot) in declared {
        let after = u32::try_from(*at).ok()?.checked_add(FIRST)?;
        *slots.get_mut(usize::try_from(*slot).ok()?)? = after;
    }
    Some(order)
}

pub(crate) fn pending(layout: &FrameLayout, declared: &[(usize, u32)]) -> Option<Provable> {
    if declared.is_empty() {
        return Some(Provable::All);
    }
    Some(Provable::Pending {
        order: declaration_order(layout.len(), declared)?,
        declared: FIRST,
    })
}

pub(crate) fn none_proven(layout: &FrameLayout) -> Provable {
    match layout.len() {
        0 => Provable::All,
        len => Provable::Pending {
            order: std::iter::repeat_n(FIRST, len).collect(),
            declared: FIRST,
        },
    }
}

fn proven_after(order: &Rc<[u32]>, index: usize, remaining: bool) -> Option<Provable> {
    if !remaining {
        return Some(Provable::All);
    }
    let declared = u32::try_from(index).ok()?.checked_add(FIRST + 1)?;
    Some(Provable::Pending {
        order: order.clone(),
        declared,
    })
}

pub(crate) fn progressive<'a>(
    statements: &'a [Statement<'a>],
    head: &Rc<ScopeChain>,
    declared: &[(usize, u32)],
) -> Box<[Lazy<'a>]> {
    let Some(order) = head.pending_order() else {
        return lazies(statements, head);
    };
    let mut chain = head.clone();
    let mut next = 0;
    let mut out = Vec::with_capacity(statements.len());
    for (index, statement) in statements.iter().enumerate() {
        out.push(Lazy::new(statement, chain.clone()));
        let before = next;
        while declared.get(next).is_some_and(|(at, _)| *at == index) {
            next += 1;
        }
        if next == before {
            continue;
        }
        if let Some(provable) = proven_after(order, index, next < declared.len()) {
            chain = head.sibling(provable);
        }
    }
    out.into_boxed_slice()
}

type Laid = (Rc<FrameLayout>, Box<[(usize, u32)]>);

pub(crate) fn lexical_layout<'a>(
    names: &mut Names<'a>,
    base: Vec<(Name, SlotKind)>,
    statements: &'a [Statement<'a>],
) -> Option<Laid> {
    let declares = statements.iter().any(|statement| {
        matches!(statement, Statement::VariableDeclaration(declaration) if is_lexical(declaration.kind))
    });
    if !declares {
        return Some((block_layout(base)?, Box::default()));
    }
    let mut builder = LayoutBuilder::new();
    for (name, kind) in base {
        builder.slot_of_kind(name, kind)?;
    }
    let declared = scan(names, &mut builder, statements)?;
    Some((builder.finish()?, declared))
}
