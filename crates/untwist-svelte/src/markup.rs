use crate::html::is_void;

#[derive(Clone, Debug)]
pub enum M {
    Element {
        name: String,
        attrs: Vec<String>,
        children: Vec<M>,
        svg: bool,
    },
    Text(String),
    Block {
        sections: Vec<(String, Vec<M>)>,
        close: String,
    },
    Tag(String),
}

const INDENT: &str = "\t";
const NO_TEXT: &[&str] = &[
    "table", "thead", "tbody", "tfoot", "tr", "colgroup", "select", "optgroup",
];
const WIDTH: usize = 100;

pub fn braces(text: &str) -> String {
    text.replace('{', "&#123;").replace('}', "&#125;")
}

pub fn escape_text(text: &str) -> String {
    braces(&text.replace('&', "&amp;").replace('<', "&lt;"))
}

fn is_space(node: &M) -> bool {
    matches!(node, M::Text(text) if text.trim().is_empty())
}

fn join(lines: &mut Vec<String>, more: Vec<String>, depth: usize) {
    let mut more = more.into_iter();
    let indent = pad(depth);
    match (lines.last_mut(), more.next()) {
        (Some(last), Some(first)) => {
            last.push_str(first.strip_prefix(indent.as_str()).unwrap_or(&first))
        }
        (None, Some(first)) => lines.push(first),
        _ => {}
    }
    lines.extend(more);
}

fn bounded(nodes: &[M]) -> Vec<M> {
    let mut nodes: Vec<M> = nodes.to_vec();
    if let Some(M::Text(first)) = nodes.first_mut() {
        if !first.trim().is_empty() && first.starts_with(char::is_whitespace) {
            *first = format!("{{' '}}{}", first.trim_start());
        }
    }
    if let Some(M::Text(last)) = nodes.last_mut() {
        if !last.trim().is_empty() && last.ends_with(char::is_whitespace) {
            *last = format!("{}{{' '}}", last.trim_end());
        }
    }
    nodes
}

pub fn lines(nodes: &[M], depth: usize) -> Vec<String> {
    let nodes = bounded(nodes);
    let mut out: Vec<String> = Vec::new();
    let mut line: Vec<String> = Vec::new();
    for node in &nodes {
        if is_space(node) {
            out.append(&mut line);
            continue;
        }
        join(&mut line, node_lines(node, depth), depth);
    }
    out.append(&mut line);
    out
}

fn pad(depth: usize) -> String {
    INDENT.repeat(depth)
}

fn inline(nodes: &[M]) -> Option<String> {
    let mut text = String::new();
    for node in nodes {
        match node {
            M::Text(part) | M::Tag(part) => text.push_str(part),
            M::Element {
                name,
                attrs,
                children,
                svg,
            } => {
                text.push_str(&open(name, attrs));
                match children.is_empty() && (is_void(name) || *svg) {
                    true => {
                        text.pop();
                        text.push_str(if attrs.is_empty() { "/>" } else { " />" });
                    }
                    false => {
                        text.push_str(&inline(children)?);
                        text.push_str(&format!("</{name}>"));
                    }
                }
            }
            M::Block { .. } => return None,
        }
    }
    Some(text)
}

fn open(name: &str, attrs: &[String]) -> String {
    match attrs.is_empty() {
        true => format!("<{name}>"),
        false => format!("<{name} {}>", attrs.join(" ")),
    }
}

fn node_lines(node: &M, depth: usize) -> Vec<String> {
    let at = pad(depth);
    match node {
        M::Text(text) | M::Tag(text) => vec![format!("{at}{text}")],
        M::Element {
            name,
            attrs,
            children,
            ..
        } if children.is_empty() || name == "pre" || name == "textarea" => {
            vec![format!(
                "{at}{}",
                inline(std::slice::from_ref(node)).unwrap_or_default()
            )]
        }
        M::Element {
            name,
            attrs,
            children,
            ..
        } if NO_TEXT.contains(&name.as_str())
            && children.iter().all(|child| !matches!(child, M::Text(_))) =>
        {
            let mut out = vec![format!("{at}{}", open(name, attrs))];
            for child in children {
                out.extend(node_lines(child, depth + 1));
            }
            out.push(format!("{at}</{name}>"));
            out
        }
        M::Element {
            name,
            attrs,
            children,
            ..
        } => {
            if let Some(text) = inline(children).filter(|text| {
                !text.contains('\n')
                    && text.len() + at.len() < WIDTH
                    && children.iter().all(|child| !is_space(child))
            }) {
                return vec![format!("{at}{}{text}</{name}>", open(name, attrs))];
            }
            let mut out = vec![format!("{at}{}", open(name, attrs))];
            out.extend(lines(children, depth + 1));
            out.push(format!("{at}</{name}>"));
            out
        }
        M::Block { sections, close } => {
            let mut out = Vec::new();
            for (header, children) in sections {
                out.push(format!("{at}{header}"));
                out.extend(lines(children, depth + 1));
            }
            out.push(format!("{at}{close}"));
            out
        }
    }
}

pub fn print(nodes: &[M]) -> String {
    let mut text = lines(nodes, 0).join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    text
}
