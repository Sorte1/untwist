use std::cell::{OnceCell, RefCell};

use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::matcher::is_literal_argument;

use super::decoder::decoder_refusal;
use super::intrinsics::{machinery_unreachable, Machinery};
use super::rotator::{uninitialised, LiteralBindings, RotatorSite};
use super::seal::sealed;
use crate::stamp::{context_is_strict, Stamped, StampedFn};

const HOST_SINK: &str = "console";

struct ForeignUnit<'r, 'p> {
    array: &'r StampedFn<'p>,
    decoders: Vec<&'r StampedFn<'p>>,
    rotators: Vec<RotatorSite>,
    proven: OnceCell<bool>,
}

impl<'r, 'p> ForeignUnit<'r, 'p> {
    fn new(array: &'r StampedFn<'p>, rotators: &[(usize, RotatorSite)]) -> Self {
        Self {
            array,
            decoders: Vec::new(),
            rotators: rotators
                .iter()
                .filter(|(owner, _)| *owner == array.number)
                .map(|(_, site)| *site)
                .collect(),
            proven: OnceCell::new(),
        }
    }

    fn allowed(&self) -> FxHashSet<SymbolId> {
        self.decoders
            .iter()
            .filter_map(|decoder| decoder.symbol)
            .chain(self.array.symbol)
            .collect()
    }

    fn parts<'s>(
        &'s self,
        program: &'s Program<'p>,
        rotators: &[&'s Statement<'p>],
    ) -> Vec<(Machinery<'s, 'p>, bool)> {
        let functions = std::iter::once(self.array)
            .chain(self.decoders.iter().copied())
            .map(|entry| {
                let strict = context_is_strict(program, entry.statement);
                (Machinery::Function(entry.function), strict)
            });
        let statements = rotators.iter().copied().map(|statement| {
            let strict = context_is_strict(program, statement.span());
            (Machinery::Statement(statement), strict)
        });
        functions.chain(statements).collect()
    }

    fn touches_only_itself(&self, program: &Program<'p>, scoping: &Scoping) -> bool {
        let allowed = self.allowed();
        if uninitialised(scoping, &allowed, &FxHashSet::default()).is_some() {
            return false;
        }
        let read_only = self
            .decoders
            .iter()
            .all(|decoder| decoder_refusal(scoping, decoder, self.array.symbol).is_none());
        if !read_only {
            return false;
        }
        let rotators: Option<Vec<&Statement<'p>>> = self
            .rotators
            .iter()
            .map(|site| site.statement(program))
            .collect();
        let Some(rotators) = rotators else {
            return false;
        };
        if rotators
            .iter()
            .any(|statement| sealed(statement, scoping, &allowed).is_some())
        {
            return false;
        }
        machinery_unreachable(scoping, &self.parts(program, &rotators))
    }
}

pub struct Reliance {
    pub units: Vec<usize>,
    pub span: Span,
}

impl Reliance {
    pub fn unheld(&self, decoding: &[usize]) -> Option<Span> {
        let held = self.units.iter().all(|unit| decoding.contains(unit));
        (!held).then_some(self.span)
    }
}

pub struct ForeignUnits<'r, 'p> {
    units: Vec<ForeignUnit<'r, 'p>>,
    decoder_units: Vec<(SymbolId, usize)>,
    rotator_units: Vec<(Span, usize)>,
    relied: RefCell<Vec<(usize, usize, Span)>>,
}

fn is_host_sink(scoping: &Scoping, expr: &Expression<'_>) -> bool {
    let Expression::Identifier(id) = expr.without_parentheses() else {
        return false;
    };
    let unresolved = id
        .reference_id
        .get()
        .is_some_and(|r| scoping.get_reference(r).symbol_id().is_none());
    unresolved && id.name.as_str() == HOST_SINK
}

impl<'r, 'p> ForeignUnits<'r, 'p> {
    pub fn new(
        stamped: &'r Stamped<'p>,
        sites: &FxHashMap<usize, (usize, usize)>,
        rotators: &[(usize, RotatorSite)],
    ) -> Self {
        let mut units: Vec<ForeignUnit<'r, 'p>> = stamped
            .arrays
            .iter()
            .map(|array| ForeignUnit::new(array, rotators))
            .collect();
        let mut decoder_units = Vec::new();
        for decoder in &stamped.decoders {
            let Some((unit, _)) = sites.get(&decoder.number) else {
                continue;
            };
            let Some(foreign) = units
                .iter_mut()
                .find(|foreign| foreign.array.number == *unit)
            else {
                continue;
            };
            foreign.decoders.push(decoder);
            if let Some(symbol) = decoder.symbol {
                decoder_units.push((symbol, *unit));
            }
        }
        Self {
            units,
            decoder_units,
            rotator_units: rotators
                .iter()
                .map(|(unit, site)| (site.span, *unit))
                .collect(),
            relied: RefCell::new(Vec::new()),
        }
    }

    fn unit_of_rotator(&self, span: Span) -> Option<usize> {
        self.rotator_units
            .iter()
            .find(|(site, _)| *site == span)
            .map(|(_, unit)| *unit)
    }

    fn unit_of_decoder(&self, symbol: SymbolId) -> Option<usize> {
        self.decoder_units
            .iter()
            .find(|(decoder, _)| *decoder == symbol)
            .map(|(_, unit)| *unit)
    }

    pub(super) fn admits(
        &self,
        program: &Program<'p>,
        scoping: &Scoping,
        literals: &LiteralBindings,
        statement: &Statement<'p>,
        judged: usize,
    ) -> bool {
        let Statement::ExpressionStatement(expression) = statement else {
            return false;
        };
        if let Some(owner) = self.unit_of_rotator(statement.span()) {
            let admitted = owner != judged && self.proven(program, scoping, owner);
            if admitted {
                self.rely(judged, &[owner], statement.span());
            }
            return admitted;
        }
        let mut owners = Vec::new();
        let shaped = match self.foreign_call(scoping, &expression.expression, judged) {
            Some(owner) => {
                owners.push(owner);
                true
            }
            None => self.host_sink(
                scoping,
                literals,
                &expression.expression,
                judged,
                &mut owners,
            ),
        };
        let admitted = shaped
            && !owners.is_empty()
            && owners
                .iter()
                .all(|owner| self.proven(program, scoping, *owner));
        if admitted {
            self.rely(judged, &owners, statement.span());
        }
        admitted
    }

    fn rely(&self, judged: usize, owners: &[usize], span: Span) {
        self.relied
            .borrow_mut()
            .extend(owners.iter().map(|owner| (judged, *owner, span)));
    }

    pub fn reliance(&self, judged: usize) -> Option<Reliance> {
        let relied = self.relied.borrow();
        let mine = || relied.iter().filter(|(unit, _, _)| *unit == judged);
        let span = mine()
            .map(|(_, _, span)| *span)
            .min_by_key(|span| span.start)?;
        let mut units: Vec<usize> = mine().map(|(_, owner, _)| *owner).collect();
        units.sort_unstable();
        units.dedup();
        Some(Reliance { units, span })
    }

    fn proven(&self, program: &Program<'p>, scoping: &Scoping, unit: usize) -> bool {
        self.units
            .iter()
            .find(|foreign| foreign.array.number == unit)
            .is_some_and(|foreign| {
                *foreign
                    .proven
                    .get_or_init(|| foreign.touches_only_itself(program, scoping))
            })
    }

    fn foreign_call(
        &self,
        scoping: &Scoping,
        expr: &Expression<'_>,
        judged: usize,
    ) -> Option<usize> {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return None;
        };
        let Expression::Identifier(callee) = call.callee.without_parentheses() else {
            return None;
        };
        let symbol = callee
            .reference_id
            .get()
            .and_then(|r| scoping.get_reference(r).symbol_id())?;
        let owner = self.unit_of_decoder(symbol)?;
        let literal = !call.arguments.is_empty()
            && call
                .arguments
                .iter()
                .all(|argument| argument.as_expression().is_some_and(is_literal_argument));
        (owner != judged && literal).then_some(owner)
    }

    fn sink_operand(
        &self,
        scoping: &Scoping,
        literals: &LiteralBindings,
        expr: &Expression<'_>,
        judged: usize,
        owners: &mut Vec<usize>,
    ) -> bool {
        if let Some(owner) = self.foreign_call(scoping, expr, judged) {
            owners.push(owner);
            return true;
        }
        literals.runs_no_code(scoping, expr)
    }

    fn sink_key(
        &self,
        scoping: &Scoping,
        member: &MemberExpression<'_>,
        judged: usize,
        owners: &mut Vec<usize>,
    ) -> bool {
        match member {
            MemberExpression::StaticMemberExpression(_) => true,
            MemberExpression::ComputedMemberExpression(computed) => {
                match computed.expression.without_parentheses() {
                    Expression::StringLiteral(_) => true,
                    key => match self.foreign_call(scoping, key, judged) {
                        Some(owner) => {
                            owners.push(owner);
                            true
                        }
                        None => false,
                    },
                }
            }
            MemberExpression::PrivateFieldExpression(_) => false,
        }
    }

    fn host_sink(
        &self,
        scoping: &Scoping,
        literals: &LiteralBindings,
        expr: &Expression<'_>,
        judged: usize,
        owners: &mut Vec<usize>,
    ) -> bool {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return false;
        };
        let Some(member) = call.callee.without_parentheses().as_member_expression() else {
            return false;
        };
        if !is_host_sink(scoping, member.object()) {
            return false;
        }
        self.sink_key(scoping, member, judged, owners)
            && call.arguments.iter().all(|argument| {
                argument.as_expression().is_some_and(|operand| {
                    self.sink_operand(scoping, literals, operand, judged, owners)
                })
            })
    }
}
