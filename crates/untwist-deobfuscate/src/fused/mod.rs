mod uses;
mod walk;

#[cfg(debug_assertions)]
#[doc(hidden)]
pub mod check;

use oxc_allocator::Vec as AVec;
use oxc_ast::ast::*;

pub(crate) use walk::Walk;

pub(crate) trait Member<'a> {
    const PRUNES: bool = false;
    type Saved: Default;

    fn enter_statements(&mut self, _it: &AVec<'a, Statement<'a>>) -> Self::Saved {
        Self::Saved::default()
    }
    fn leave_statements(&mut self, _saved: Self::Saved) {}
    fn enter_statement(&mut self, _it: &Statement<'a>) -> Self::Saved {
        Self::Saved::default()
    }
    fn leave_statement(&mut self, _saved: Self::Saved) {}
    fn enter_expression(&mut self, _it: &Expression<'a>) -> Option<Self::Saved> {
        Some(Self::Saved::default())
    }
    fn leave_expression(&mut self, _saved: Self::Saved) {}
    fn identifier_reference(&mut self, _it: &IdentifierReference<'a>) {}
    fn binding_identifier(&mut self, _it: &BindingIdentifier<'a>) {}
    fn assignment_expression(&mut self, _it: &AssignmentExpression<'a>) {}
    fn enter_variable_declarator(&mut self, _it: &VariableDeclarator<'a>) -> Self::Saved {
        Self::Saved::default()
    }
    fn leave_variable_declarator(&mut self, _saved: Self::Saved) {}
    fn formal_parameter(&mut self, _it: &FormalParameter<'a>) {}
    fn catch_parameter(&mut self, _it: &CatchParameter<'a>) {}
    fn enter_function(&mut self, _it: &Function<'a>) -> Self::Saved {
        Self::Saved::default()
    }
    fn leave_function(&mut self, _saved: Self::Saved) {}
    fn enter_arrow(&mut self, _it: &ArrowFunctionExpression<'a>) -> Self::Saved {
        Self::Saved::default()
    }
    fn leave_arrow(&mut self, _saved: Self::Saved) {}
    fn enter_class(&mut self, _it: &Class<'a>) -> Self::Saved {
        Self::Saved::default()
    }
    fn leave_class(&mut self, _saved: Self::Saved) {}
    fn enter_class_heritage(&mut self) -> Self::Saved {
        Self::Saved::default()
    }
    fn leave_class_heritage(&mut self, _saved: Self::Saved) {}
    fn enter_class_element(&mut self) -> Self::Saved {
        Self::Saved::default()
    }
    fn leave_class_element(&mut self, _saved: Self::Saved) {}
    fn enter_property_key(&mut self) -> Self::Saved {
        Self::Saved::default()
    }
    fn leave_property_key(&mut self, _saved: Self::Saved) {}
    fn enter_nested(&mut self) -> Self::Saved {
        Self::Saved::default()
    }
    fn leave_nested(&mut self, _saved: Self::Saved) {}
}

#[derive(Default)]
pub(crate) enum Slot<S> {
    #[default]
    Off,
    Pruned,
    On(S),
}

pub(crate) struct Fused<A, B> {
    pub(crate) first: A,
    pub(crate) second: B,
    first_muted: bool,
    second_muted: bool,
}

impl<A, B> Fused<A, B> {
    pub(crate) fn new(first: A, second: B) -> Self {
        Self {
            first,
            second,
            first_muted: false,
            second_muted: false,
        }
    }
}

macro_rules! fused_hooks {
    ($($enter:ident, $leave:ident($($arg:ident: $ty:ty),*);)*) => {
        $(
            #[inline(always)]
            fn $enter(&mut self, $($arg: $ty),*) -> Self::Saved {
                let first = self.first_on().then(|| self.first.$enter($($arg),*));
                let second = self.second_on().then(|| self.second.$enter($($arg),*));
                (first.map_or(Slot::Off, Slot::On), second.map_or(Slot::Off, Slot::On))
            }

            #[inline(always)]
            fn $leave(&mut self, (first, second): Self::Saved) {
                if let Slot::On(saved) = second {
                    self.second.$leave(saved);
                }
                if let Slot::On(saved) = first {
                    self.first.$leave(saved);
                }
            }
        )*
    };
}

impl<'a, A: Member<'a>, B: Member<'a>> Fused<A, B> {
    #[inline(always)]
    fn first_on(&self) -> bool {
        !A::PRUNES || !self.first_muted
    }

    #[inline(always)]
    fn second_on(&self) -> bool {
        !B::PRUNES || !self.second_muted
    }

    #[inline(always)]
    fn enter_first(&mut self, it: &Expression<'a>) -> Slot<A::Saved> {
        if !self.first_on() {
            return Slot::Off;
        }
        match self.first.enter_expression(it) {
            Some(saved) => Slot::On(saved),
            None => {
                self.first_muted = true;
                Slot::Pruned
            }
        }
    }

    #[inline(always)]
    fn enter_second(&mut self, it: &Expression<'a>) -> Slot<B::Saved> {
        if !self.second_on() {
            return Slot::Off;
        }
        match self.second.enter_expression(it) {
            Some(saved) => Slot::On(saved),
            None => {
                self.second_muted = true;
                Slot::Pruned
            }
        }
    }

    #[inline(always)]
    fn unmute(&mut self, first: &Slot<A::Saved>, second: &Slot<B::Saved>) {
        if matches!(first, Slot::Pruned) {
            self.first_muted = false;
        }
        if matches!(second, Slot::Pruned) {
            self.second_muted = false;
        }
    }
}

impl<'a, A: Member<'a>, B: Member<'a>> Member<'a> for Fused<A, B> {
    type Saved = (Slot<A::Saved>, Slot<B::Saved>);

    fused_hooks! {
        enter_statements, leave_statements(it: &AVec<'a, Statement<'a>>);
        enter_statement, leave_statement(it: &Statement<'a>);
        enter_variable_declarator, leave_variable_declarator(it: &VariableDeclarator<'a>);
        enter_function, leave_function(it: &Function<'a>);
        enter_arrow, leave_arrow(it: &ArrowFunctionExpression<'a>);
        enter_class, leave_class(it: &Class<'a>);
        enter_class_heritage, leave_class_heritage();
        enter_class_element, leave_class_element();
        enter_property_key, leave_property_key();
        enter_nested, leave_nested();
    }

    #[inline(always)]
    fn enter_expression(&mut self, it: &Expression<'a>) -> Option<Self::Saved> {
        let first = self.enter_first(it);
        let second = self.enter_second(it);
        Some((first, second))
    }

    #[inline(always)]
    fn leave_expression(&mut self, (first, second): Self::Saved) {
        self.unmute(&first, &second);
        if let Slot::On(saved) = second {
            self.second.leave_expression(saved);
        }
        if let Slot::On(saved) = first {
            self.first.leave_expression(saved);
        }
    }

    #[inline(always)]
    fn identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.first_on() {
            self.first.identifier_reference(it);
        }
        if self.second_on() {
            self.second.identifier_reference(it);
        }
    }

    #[inline(always)]
    fn binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if self.first_on() {
            self.first.binding_identifier(it);
        }
        if self.second_on() {
            self.second.binding_identifier(it);
        }
    }

    #[inline(always)]
    fn assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if self.first_on() {
            self.first.assignment_expression(it);
        }
        if self.second_on() {
            self.second.assignment_expression(it);
        }
    }

    #[inline(always)]
    fn formal_parameter(&mut self, it: &FormalParameter<'a>) {
        if self.first_on() {
            self.first.formal_parameter(it);
        }
        if self.second_on() {
            self.second.formal_parameter(it);
        }
    }

    #[inline(always)]
    fn catch_parameter(&mut self, it: &CatchParameter<'a>) {
        if self.first_on() {
            self.first.catch_parameter(it);
        }
        if self.second_on() {
            self.second.catch_parameter(it);
        }
    }
}
