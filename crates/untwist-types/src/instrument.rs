use oxc_ast::AstKind;
use oxc_span::GetSpan;

use crate::emit::{apply, Edit};
use crate::facts::SlotKind;
use crate::infer::Env;
use crate::ty::Ty;
use crate::view::Annotations;

const PRELUDE_HEAD: &str = "var __ut = (function () {\n  var stderr = typeof process === \"object\" && process && process.stderr;\n  var failed = {};\n  var checks = [\n";
const PRELUDE_TAIL: &str = "  ];\n  function __ut(v, id) {\n    if (!checks[id](v) && !failed[id]) {\n      failed[id] = true;\n      if (stderr) stderr.write(\"__UT_FAIL \" + id + \"\\n\");\n    }\n    return v;\n  }\n  return __ut;\n})();\n";

struct Checks {
    predicates: Vec<String>,
    edits: Vec<Edit>,
}

impl Checks {
    fn id(&mut self, ty: &Ty) -> usize {
        self.predicates.push(format!(
            "    function (v) {{ return {}; }},\n",
            ty.predicate("v")
        ));
        self.predicates.len() - 1
    }

    fn wrap(&mut self, start: u32, end: u32, id: usize) {
        let length = end - start;
        self.edits.push(Edit {
            at: start,
            rank: 0,
            tie: u32::MAX - length,
            text: "__ut((".to_string(),
        });
        self.edits.push(Edit {
            at: end,
            rank: 9,
            tie: length,
            text: format!("), {id})"),
        });
    }
}

fn reads_wrappable(kind: AstKind<'_>) -> bool {
    !matches!(
        kind,
        AstKind::ExportSpecifier(_)
            | AstKind::JSXMemberExpression(_)
            | AstKind::JSXOpeningElement(_)
            | AstKind::JSXClosingElement(_)
    ) && !matches!(kind, AstKind::UnaryExpression(unary) if unary.operator == oxc_syntax::operator::UnaryOperator::Delete)
}

pub fn instrument(env: &Env<'_, '_>, notes: &Annotations, code: &str) -> String {
    let Some(first) = env.cx.program.body.first() else {
        return code.to_string();
    };
    let mut checks = Checks {
        predicates: Vec::new(),
        edits: Vec::new(),
    };
    let mut symbols: Vec<_> = notes.symbols.iter().collect();
    symbols.sort_by_key(|(symbol, _)| symbol.index());
    for (symbol, ty) in symbols {
        let id = checks.id(ty);
        if let Some(slot) = env.facts.slots.get(symbol) {
            if let (SlotKind::Binding { .. }, Some(init)) =
                (slot.kind, slot.declarator.and_then(|d| d.init.as_ref()))
            {
                let span = init.span();
                checks.wrap(span.start, span.end, id);
            }
        }
        for reference in env.cx.scoping.get_resolved_references(*symbol) {
            if reference.flags().is_write() {
                continue;
            }
            let node = reference.node_id();
            let parent = env.cx.nodes.parent_kind(node);
            if !reads_wrappable(parent) {
                continue;
            }
            let span = env.cx.nodes.kind(node).span();
            if let AstKind::ObjectProperty(property) = parent {
                if property.shorthand {
                    let name = env.cx.scoping.symbol_name(*symbol);
                    checks.edits.push(Edit {
                        at: span.start,
                        rank: 0,
                        tie: 0,
                        text: format!("{name}: "),
                    });
                }
            }
            checks.wrap(span.start, span.end, id);
        }
    }
    let mut returns: Vec<_> = notes.returns.iter().collect();
    returns.sort_by_key(|(index, _)| **index);
    for (index, ty) in returns {
        let fact = &env.facts.functions[*index];
        let inner = match (fact.func.is_async(), ty.members()) {
            (true, [crate::ty::Member::Promise(inner)]) => (**inner).clone(),
            (true, _) => continue,
            (false, _) => ty.clone(),
        };
        let id = checks.id(&inner);
        let operands = fact
            .returns
            .iter()
            .flatten()
            .copied()
            .chain(fact.func.expression_body());
        for operand in operands {
            let span = operand.span();
            checks.wrap(span.start, span.end, id);
        }
    }
    let prelude = format!("{PRELUDE_HEAD}{}{PRELUDE_TAIL}", checks.predicates.concat());
    checks.edits.push(Edit {
        at: first.span().start,
        rank: 0,
        tie: 0,
        text: prelude,
    });
    apply(code, checks.edits)
}
