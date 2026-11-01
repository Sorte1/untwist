use oxc_allocator::Allocator;
use oxc_ast::ast::*;

use super::layout::module_id;
use super::CANONICAL;
use crate::rename::rename_parameters;
use crate::runtime::js_string;

pub fn dependencies(deps: Option<&ArrayExpression<'_>>) -> Vec<(String, String)> {
    let elements = deps.map_or(&[][..], |deps| deps.elements.as_slice());
    elements
        .iter()
        .filter_map(|element| {
            let expression = element.as_expression()?;
            let id = module_id(expression)?;
            let text = match expression {
                Expression::NumericLiteral(_) => id.clone(),
                _ => js_string(&id),
            };
            Some((text, id))
        })
        .collect()
}

pub fn normalize<'a>(alloc: &'a Allocator, program: &mut Program<'a>, names: &[&str]) {
    if names != CANONICAL {
        rename_parameters(alloc, program, names, &CANONICAL);
    }
}
