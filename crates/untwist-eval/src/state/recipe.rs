mod compile;
mod prim;
#[cfg(test)]
mod tests;

use oxc_span::Span;

use crate::names::Name;
use crate::value::{ObjKind, ObjRef, PropKey, Value};

pub(crate) use compile::compile_recipe;
pub(crate) use prim::{fold_constant, Prim};

#[derive(Clone, Debug)]
pub(crate) enum KeyTerm {
    Argument(u8),
    Literal(Value<'static>),
    ArrayAt(Box<KeyTerm>),
    Add(Box<KeyTerm>, Box<KeyTerm>),
    Sub(Box<KeyTerm>, Box<KeyTerm>),
}

impl PartialEq for KeyTerm {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (KeyTerm::Argument(a), KeyTerm::Argument(b)) => a == b,
            (KeyTerm::Literal(a), KeyTerm::Literal(b)) => match (Prim::of(a), Prim::of(b)) {
                (Some(a), Some(b)) => a.same(&b),
                _ => false,
            },
            (KeyTerm::ArrayAt(a), KeyTerm::ArrayAt(b)) => a == b,
            (KeyTerm::Add(a, b), KeyTerm::Add(c, d)) | (KeyTerm::Sub(a, b), KeyTerm::Sub(c, d)) => {
                a == c && b == d
            }
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ContainerRef {
    Binding(Name),
    SelfMember(PropKey),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GateTest {
    Undefined { loose: bool },
    Falsy,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GateRef {
    pub(crate) span: Span,
    pub(crate) flag: PropKey,
    pub(crate) test: GateTest,
}

#[derive(Clone, Debug)]
pub(crate) struct KeyRecipe {
    pub(crate) container: ContainerRef,
    pub(crate) key: KeyTerm,
    pub(crate) gate: Option<GateRef>,
    pub(crate) array: Option<Name>,
    pub(crate) reads: Vec<KeyTerm>,
    #[cfg(test)]
    pub(crate) slice: Vec<Span>,
    pub(crate) self_name: Name,
    pub(crate) undefined: Name,
}

impl KeyRecipe {
    pub(crate) fn reads_self(&self) -> bool {
        self.gate.is_some() || matches!(self.container, ContainerRef::SelfMember(_))
    }
}

pub(crate) fn apply_recipe<'a>(
    recipe: &KeyRecipe,
    args: &[Value<'a>],
    array: &ObjRef<'a>,
) -> Option<PropKey> {
    let borrowed = array.borrow();
    let ObjKind::Array(items) = &borrowed.kind else {
        return None;
    };
    let scope = Scope { args, items };
    for read in &recipe.reads {
        scope.eval(read)?;
    }
    Some(scope.eval(&recipe.key)?.key())
}

struct Scope<'s, 'a> {
    args: &'s [Value<'a>],
    items: &'s [Value<'a>],
}

impl Scope<'_, '_> {
    fn eval(&self, term: &KeyTerm) -> Option<Prim> {
        match term {
            KeyTerm::Argument(index) => Prim::of(self.args.get(usize::from(*index))?),
            KeyTerm::Literal(value) => Prim::of(value),
            KeyTerm::ArrayAt(index) => self.element(&self.eval(index)?),
            KeyTerm::Add(left, right) => prim::add(&self.eval(left)?, &self.eval(right)?),
            KeyTerm::Sub(left, right) => Some(Prim::Num(
                self.eval(left)?.number() - self.eval(right)?.number(),
            )),
        }
    }

    fn element(&self, index: &Prim) -> Option<Prim> {
        let Prim::Num(number) = index else {
            return None;
        };
        let in_range = *number >= 0.0 && number.fract() == 0.0 && *number < self.items.len() as f64;
        if !in_range {
            return None;
        }
        Prim::of(self.items.get(*number as usize)?)
    }
}
