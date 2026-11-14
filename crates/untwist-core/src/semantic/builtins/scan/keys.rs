use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::operator::BinaryOperator;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashMap;

const EXACT_INTEGER: f64 = 9_007_199_254_740_991.0;

pub(super) type ConstKeys = FxHashMap<SymbolId, String>;

pub(super) fn literal_text(expression: &Expression<'_>) -> Option<String> {
    match expression.without_parentheses() {
        Expression::StringLiteral(literal) => literal.value.as_str().map(str::to_string),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked.as_ref())
            .and_then(|cooked| cooked.as_str())
            .map(str::to_string),
        Expression::NumericLiteral(literal) => number_text(literal.value),
        Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
            let left = literal_text(&binary.left)?;
            let right = literal_text(&binary.right)?;
            let numbers = is_number(&binary.left) && is_number(&binary.right);
            (!numbers).then(|| left + &right)
        }
        _ => None,
    }
}

fn is_number(expression: &Expression<'_>) -> bool {
    matches!(
        expression.without_parentheses(),
        Expression::NumericLiteral(_)
    )
}

fn number_text(value: f64) -> Option<String> {
    let exact = value.fract() == 0.0 && value.abs() <= EXACT_INTEGER;
    match exact {
        true if value == 0.0 => Some("0".to_string()),
        true => Some(format!("{}", value as i64)),
        false => None,
    }
}

pub(super) fn const_keys(program: &Program<'_>, scoping: &oxc_semantic::Scoping) -> ConstKeys {
    let mut found = Consts {
        scoping,
        keys: FxHashMap::default(),
    };
    found.visit_program(program);
    found.keys
}

struct Consts<'s> {
    scoping: &'s oxc_semantic::Scoping,
    keys: ConstKeys,
}

impl<'a> VisitJs<'a> for Consts<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            let symbol = id.symbol_id.get().filter(|symbol| {
                self.scoping
                    .symbol_flags(*symbol)
                    .contains(SymbolFlags::ConstVariable)
            });
            if let (Some(symbol), Some(text)) = (symbol, literal_text(init)) {
                self.keys.insert(symbol, text);
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }
}
