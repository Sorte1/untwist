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
    matches!(node, M::Text(text) if !text.is_empty() && text.chars().all(is_ws))
}

fn is_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Edge,
    Expr,
    Other,
}

fn side(node: Option<&M>) -> Side {
    match node {
        None => Side::Edge,
        Some(M::Tag(text))
            if text.starts_with('{')
                && !text.starts_with("{@")
                && !text.starts_with("{#")
                && !text.starts_with("{:")
                && !text.starts_with("{/") =>
        {
            Side::Expr
        }
        Some(_) => Side::Other,
    }
}

fn entity(text: &str) -> Option<(char, usize)> {
    let body = text.strip_prefix("&#")?;
    let end = body.find(';')?;
    let digits = &body[..end];
    let code = match digits.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => digits.parse().ok()?,
    };
    Some((char::from_u32(code)?, end + 3))
}

fn lead_ws(text: &str) -> (String, usize) {
    let mut data = String::new();
    let mut at = 0;
    while at < text.len() {
        let rest = &text[at..];
        if let Some((c, len)) = entity(rest).filter(|(c, _)| is_ws(*c)) {
            data.push(c);
            at += len;
            continue;
        }
        match rest.chars().next() {
            Some(c) if is_ws(c) => {
                data.push(c);
                at += c.len_utf8();
            }
            _ => break,
        }
    }
    (data, at)
}

fn trail_ws(text: &str) -> (String, usize) {
    let mut data: Vec<char> = Vec::new();
    let mut end = text.len();
    loop {
        let rest = &text[..end];
        if let Some(c) = rest.chars().next_back().filter(|c| is_ws(*c)) {
            data.push(c);
            end -= c.len_utf8();
            continue;
        }
        if rest.ends_with(';') {
            if let Some(start) = rest.rfind("&#") {
                if let Some((c, len)) =
                    entity(&rest[start..]).filter(|(c, len)| is_ws(*c) && start + len == end)
                {
                    data.push(c);
                    end -= len;
                    continue;
                }
            }
        }
        break;
    }
    (data.into_iter().rev().collect(), text.len() - end)
}

fn ws_tag(data: &str) -> M {
    let mut out = String::from("{'");
    for c in data.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out.push_str("'}");
    M::Tag(out)
}

fn kept(data: &str, before: Side, after: Side) -> String {
    let mut text = data.to_string();
    if before == Side::Other {
        text = format!(" {}", text.trim_start_matches(is_ws));
    }
    if after == Side::Other {
        text = format!("{} ", text.trim_end_matches(is_ws));
    }
    text
}

fn guard(nodes: &[M]) -> Vec<M> {
    let mut out = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        let M::Text(text) = node else {
            out.push(node.clone());
            continue;
        };
        let before = side(index.checked_sub(1).and_then(|at| nodes.get(at)));
        let after = side(nodes.get(index + 1));
        let (lead, lead_len) = lead_ws(text);
        if lead_len == text.len() {
            if lead.is_empty() {
                continue;
            }
            let fine =
                before != Side::Edge && after != Side::Edge && kept(&lead, before, after) == lead;
            match (fine, before == Side::Other && after == Side::Other) {
                (true, true) => out.push(node.clone()),
                (true, false) => out.push(M::Tag(text.clone())),
                (false, _) => out.push(ws_tag(&lead)),
            }
            continue;
        }
        let (trail, trail_len) = trail_ws(text);
        let mut core = text.as_str();
        let lead_fine = match before {
            Side::Edge => lead.is_empty(),
            Side::Expr => true,
            Side::Other => lead.is_empty() || lead == " ",
        };
        if !lead_fine {
            out.push(ws_tag(&lead));
            core = &core[lead_len..];
        }
        let trail_fine = match after {
            Side::Edge => trail.is_empty(),
            Side::Expr => true,
            Side::Other => trail.is_empty() || trail == " ",
        };
        match trail_fine {
            true => out.push(M::Text(core.to_string())),
            false => {
                out.push(M::Text(core[..core.len() - trail_len].to_string()));
                out.push(ws_tag(&trail));
            }
        }
    }
    out
}

fn preserved(name: &str, nodes: &[M]) -> Vec<M> {
    let mut nodes = nodes.to_vec();
    if let Some(M::Text(first)) = nodes.first_mut() {
        if first.starts_with('\n') && name == "textarea" {
            first.insert(0, '\n');
        } else if let Some(rest) = first.strip_prefix('\n') {
            *first = rest.to_string();
            nodes.insert(0, ws_tag("\n"));
        }
    }
    nodes
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

pub fn lines(nodes: &[M], depth: usize) -> Vec<String> {
    let nodes = guard(nodes);
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

fn inline(nodes: &[M], raw: bool) -> Option<String> {
    inline_nodes(&guard(nodes), raw)
}

fn inline_nodes(nodes: &[M], raw: bool) -> Option<String> {
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
                    false if name == "pre" || name == "textarea" => {
                        text.push_str(&inline_nodes(&preserved(name, children), true)?);
                        text.push_str(&format!("</{name}>"));
                    }
                    false => {
                        text.push_str(&inline(children, raw)?);
                        text.push_str(&format!("</{name}>"));
                    }
                }
            }
            M::Block { sections, close } if raw => {
                for (header, children) in sections {
                    text.push_str(header);
                    text.push_str(&inline_nodes(children, true)?);
                }
                text.push_str(close);
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
                inline_nodes(std::slice::from_ref(node), false).unwrap_or_default()
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
            if let Some(text) = inline(children, false).filter(|text| {
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
