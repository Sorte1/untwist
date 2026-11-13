use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};

use crate::matcher::literal_id;
use crate::model::{Bundle, BundleKind};
use crate::notes::{chunk_computed, chunk_missing, module_missing};
use crate::webpack::gate::module_require;

const ENSURE: &str = "e";

struct Loads<'n> {
    require: &'n str,
    ids: Vec<Option<String>>,
    required: Vec<String>,
}

fn first_literal(arguments: &[Argument<'_>], at: usize) -> Option<String> {
    arguments.get(at)?.as_expression().and_then(literal_id)
}

impl<'a> VisitJs<'a> for Loads<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let is_require = |expr: &Expression<'_>| matches!(expr.without_parentheses(), Expression::Identifier(id) if id.name == self.require);
        if is_require(&it.callee) {
            self.required.extend(first_literal(&it.arguments, 0));
        }
        if let Expression::StaticMemberExpression(member) = it.callee.without_parentheses() {
            let bound = member.property.name == "bind" && is_require(&member.object);
            let fake = member.property.name == "t" && is_require(&member.object);
            if bound {
                self.required.extend(first_literal(&it.arguments, 1));
            }
            if fake {
                self.required.extend(first_literal(&it.arguments, 0));
            }
            let bound_fake = member.property.name == "bind"
                && matches!(member.object.without_parentheses(), Expression::StaticMemberExpression(inner)
                    if inner.property.name == "t" && is_require(&inner.object));
            if bound_fake {
                self.required.extend(first_literal(&it.arguments, 1));
            }
        }
        if let Expression::StaticMemberExpression(member) = it.callee.without_parentheses() {
            let ensure = member.property.name == ENSURE
                && matches!(&member.object, Expression::Identifier(id) if id.name == self.require);
            if ensure {
                let id = match &it.arguments[..] {
                    [only] => only.as_expression().and_then(literal_id),
                    _ => None,
                };
                self.ids.push(id);
            }
        }
        walk_js::walk_call_expression(self, it);
    }
}

pub fn chunks_supplied(bundle: &Bundle<'_>) -> Result<(), String> {
    if bundle.kind != BundleKind::Webpack {
        return Ok(());
    }
    let supplied = |chunk: &String| {
        bundle.chunk_ids.contains(chunk) || bundle.installed_chunks.contains(chunk)
    };
    let started = bundle.runtime.is_some();
    if let Some(chunk) = bundle
        .startup_chunks
        .iter()
        .find(|chunk| started && !supplied(chunk))
    {
        return Err(chunk_missing(
            bundle.entry_id.as_deref().unwrap_or("entry"),
            chunk,
        ));
    }
    for (id, module) in &bundle.modules {
        let Some(require) = module_require(bundle, id, module) else {
            continue;
        };
        let mut loads = Loads {
            require,
            ids: Vec::new(),
            required: Vec::new(),
        };
        loads.visit_program(module.program());
        for chunk in loads.ids {
            let Some(chunk) = chunk else {
                return Err(chunk_computed(id));
            };
            if !supplied(&chunk) {
                return Err(chunk_missing(id, &chunk));
            }
        }
        let merged = !bundle.chunk_ids.is_empty() && bundle.runtime.is_some();
        if let Some(missing) = loads
            .required
            .iter()
            .find(|required| merged && !bundle.modules.contains_key(required.as_str()))
        {
            return Err(module_missing(id, missing));
        }
    }
    Ok(())
}
