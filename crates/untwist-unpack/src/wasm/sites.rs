use oxc_ast::ast::{ArrayExpression, ArrayExpressionElement, Expression, PropertyKey};
use oxc_ast::AstKind;
use oxc_semantic::AstNodes;
use oxc_span::{GetSpan, Span};
use oxc_syntax::node::NodeId;
use oxc_syntax::operator::BinaryOperator;

use super::payload::{
    base64_holds_module, check_header, check_size, decode_text, wasm_base64, PayloadRefusal,
    MIN_MODULE_BYTES, WASM_MAGIC,
};

const HASHED_SUFFIX: &str = ".module.wasm";
const MIN_HASH_LEN: usize = 8;
const URL_CALLEES: &[&str] = &[
    "fetch",
    "locateFile",
    "instantiateStreaming",
    "compileStreaming",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PayloadSite {
    pub span: Span,
    pub bytes: Result<Vec<u8>, PayloadRefusal>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteSite {
    pub url: String,
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scan {
    pub payloads: Vec<PayloadSite>,
    pub remotes: Vec<RemoteSite>,
    pub hashes: Vec<String>,
    pub hashed_suffix: bool,
}

pub fn url_name(url: &str) -> &str {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    path.rsplit('/').next().unwrap_or(path)
}

fn is_wasm_url(value: &str) -> bool {
    let name = url_name(value);
    !value.starts_with("data:")
        && name.len() > ".wasm".len()
        && name.ends_with(".wasm")
        && !name.starts_with('.')
        && !name.contains(char::is_whitespace)
}

fn is_hash(value: &str) -> bool {
    value.len() >= MIN_HASH_LEN && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn array_byte(element: &ArrayExpressionElement<'_>) -> Option<u8> {
    let ArrayExpressionElement::NumericLiteral(number) = element else {
        return None;
    };
    let value = number.value;
    (value.fract() == 0.0 && (0.0..=255.0).contains(&value)).then_some(value as u8)
}

fn starts_with_magic(array: &ArrayExpression<'_>) -> bool {
    array.elements.len() >= WASM_MAGIC.len()
        && WASM_MAGIC
            .iter()
            .zip(&array.elements)
            .all(|(byte, element)| array_byte(element) == Some(*byte))
}

fn array_bytes(array: &ArrayExpression<'_>) -> Result<Vec<u8>, PayloadRefusal> {
    check_size(array.elements.len())?;
    let bytes: Option<Vec<u8>> = array.elements.iter().map(array_byte).collect();
    let bytes = bytes.ok_or_else(|| {
        PayloadRefusal::Malformed("an array element is not a byte literal".into())
    })?;
    check_header(&bytes).map(|()| bytes)
}

fn is_typed_array_wrapper(kind: AstKind<'_>, array: Span) -> Option<Span> {
    let (callee, arguments, span) = match kind {
        AstKind::NewExpression(new) => (&new.callee, &new.arguments, new.span),
        AstKind::CallExpression(call) => (&call.callee, &call.arguments, call.span),
        _ => return None,
    };
    let names_the_type = match callee.without_parentheses() {
        Expression::Identifier(id) => {
            id.name == "Uint8Array" && matches!(kind, AstKind::NewExpression(_))
        }
        expression => expression
            .as_member_expression()
            .is_some_and(|member| member.is_specific_member_access("Uint8Array", "from")),
    };
    let only_argument = arguments.len() == 1 && arguments[0].span() == array;
    (names_the_type && only_argument).then_some(span)
}

fn is_property_key(kind: AstKind<'_>, span: Span) -> bool {
    match kind {
        AstKind::ObjectProperty(property) => property.key.span() == span,
        _ => false,
    }
}

fn callee_name<'a>(callee: &'a Expression<'a>) -> Option<&'a str> {
    match callee.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.as_str()),
        expression => expression.as_member_expression()?.static_property_name(),
    }
}

fn names_wasm(name: &str) -> bool {
    name.to_ascii_lowercase().contains("wasm")
}

fn target_name<'a>(kind: AstKind<'a>) -> Option<&'a str> {
    match kind {
        AstKind::VariableDeclarator(declarator) => declarator
            .id
            .get_binding_identifier()
            .map(|id| id.name.as_str()),
        AstKind::AssignmentExpression(assign) => match &assign.left {
            oxc_ast::ast::AssignmentTarget::AssignmentTargetIdentifier(id) => {
                Some(id.name.as_str())
            }
            target => target.as_member_expression()?.static_property_name(),
        },
        AstKind::ObjectProperty(property) => match &property.key {
            PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
            PropertyKey::StringLiteral(text) => Some(text.value.as_str()),
            _ => None,
        },
        _ => None,
    }
}

fn in_url_context(nodes: &AstNodes<'_>, id: NodeId) -> bool {
    for kind in nodes.ancestor_kinds(id) {
        match kind {
            AstKind::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {}
            AstKind::TemplateLiteral(_) | AstKind::ParenthesizedExpression(_) => {}
            AstKind::CallExpression(call) => {
                return callee_name(&call.callee).is_some_and(|name| URL_CALLEES.contains(&name))
            }
            AstKind::NewExpression(new) => {
                return matches!(new.callee.without_parentheses(), Expression::Identifier(id) if id.name == "URL")
            }
            kind => return target_name(kind).is_some_and(names_wasm),
        }
    }
    false
}

fn hashed_operands(binary: &oxc_ast::ast::BinaryExpression<'_>, hashes: &mut Vec<String>) {
    let mut left = binary.left.without_parentheses();
    loop {
        let (operand, rest) = match left {
            Expression::BinaryExpression(inner) if inner.operator == BinaryOperator::Addition => {
                (inner.right.without_parentheses(), Some(&inner.left))
            }
            other => (other, None),
        };
        match operand {
            Expression::StringLiteral(text) if is_hash(&text.value) => {
                hashes.push(text.value.to_string());
                return;
            }
            Expression::ComputedMemberExpression(member) => {
                let mut object = member.object.without_parentheses();
                while let Expression::ComputedMemberExpression(inner) = object {
                    object = inner.object.without_parentheses();
                }
                if let Expression::ObjectExpression(map) = object {
                    map_hashes(map, hashes);
                }
                return;
            }
            Expression::StringLiteral(text) if text.value.is_empty() => {}
            _ => return,
        }
        match rest {
            Some(rest) => left = rest.without_parentheses(),
            None => return,
        }
    }
}

fn map_hashes(map: &oxc_ast::ast::ObjectExpression<'_>, hashes: &mut Vec<String>) {
    for property in map
        .properties
        .iter()
        .filter_map(|property| property.as_property())
    {
        match property.value.without_parentheses() {
            Expression::StringLiteral(text) if is_hash(&text.value) => {
                hashes.push(text.value.to_string());
            }
            Expression::ObjectExpression(nested) => map_hashes(nested, hashes),
            _ => {}
        }
    }
}

fn is_hashed_suffix(expression: &Expression<'_>) -> bool {
    matches!(expression.without_parentheses(), Expression::StringLiteral(text) if text.value == HASHED_SUFFIX)
}

impl Scan {
    fn string(&mut self, nodes: &AstNodes<'_>, id: NodeId, span: Span, value: &str) {
        if value == HASHED_SUFFIX {
            self.hashed_suffix = true;
            return;
        }
        if is_property_key(nodes.parent_kind(id), span) {
            return;
        }
        if wasm_base64(value).is_some_and(|body| !base64_holds_module(body)) {
            return;
        }
        if let Some(bytes) = decode_text(value) {
            self.payloads.push(PayloadSite { span, bytes });
            return;
        }
        if is_wasm_url(value) && in_url_context(nodes, id) {
            self.remote(value);
        }
    }

    fn remote(&mut self, url: &str) {
        self.remotes.push(RemoteSite {
            url: url.to_string(),
            name: url_name(url).to_string(),
        });
    }

    fn array(&mut self, nodes: &AstNodes<'_>, id: NodeId, array: &ArrayExpression<'_>) {
        if array.elements.len() < MIN_MODULE_BYTES || !starts_with_magic(array) {
            return;
        }
        let span = is_typed_array_wrapper(nodes.parent_kind(id), array.span).unwrap_or(array.span);
        self.payloads.push(PayloadSite {
            span,
            bytes: array_bytes(array),
        });
    }

    fn call(&mut self, call: &oxc_ast::ast::CallExpression<'_>) {
        let Some(member) = call.callee.without_parentheses().as_member_expression() else {
            return;
        };
        if member.static_property_name() != Some("v") || call.arguments.len() < 3 {
            return;
        }
        if let Some(Expression::StringLiteral(text)) = call.arguments[2].as_expression() {
            if is_hash(&text.value) {
                self.hashes.push(text.value.to_string());
            }
        }
    }

    pub fn of(nodes: &AstNodes<'_>) -> Self {
        let mut scan = Scan::default();
        let mut hashed = Vec::new();
        for node in nodes.iter() {
            match node.kind() {
                AstKind::StringLiteral(literal) => {
                    scan.string(nodes, node.id(), literal.span, &literal.value)
                }
                AstKind::TemplateLiteral(template) => {
                    if let Some(cooked) = template.single_quasi() {
                        scan.string(nodes, node.id(), template.span, &cooked);
                    }
                }
                AstKind::ArrayExpression(array) => scan.array(nodes, node.id(), array),
                AstKind::CallExpression(call) => scan.call(call),
                AstKind::BinaryExpression(binary)
                    if binary.operator == BinaryOperator::Addition
                        && is_hashed_suffix(&binary.right) =>
                {
                    hashed_operands(binary, &mut hashed);
                }
                _ => {}
            }
        }
        for hash in hashed {
            scan.remote(&format!("{hash}{HASHED_SUFFIX}"));
        }
        scan.payloads.sort_by_key(|site| site.span.start);
        scan.payloads
            .dedup_by(|later, earlier| earlier.span.end > later.span.start);
        scan
    }
}
