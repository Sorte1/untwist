#[derive(Clone, Debug)]
pub enum Child {
    Html(String),
    Str(String),
    Expr(String),
    Jsx(String),
}

const INDENT: &str = "  ";
const WIDTH: usize = 100;

pub fn escape_html_text(text: &str) -> String {
    text.replace('{', "&#123;")
        .replace('}', "&#125;")
        .replace('>', "&gt;")
}

pub fn escape_str_text(text: &str) -> String {
    escape_html_text(&text.replace('&', "&amp;").replace('<', "&lt;"))
}

pub fn js_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn plain_space(text: &str) -> bool {
    !text.chars().any(|c| c.is_whitespace() && c != ' ') && !text.contains("  ")
}

pub fn indent_rest(text: &str, indent: &str) -> String {
    let mut lines = text.split('\n');
    let mut out = lines.next().unwrap_or_default().to_string();
    let rest: Vec<&str> = lines.collect();
    let common = rest
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    for line in rest {
        out.push('\n');
        if line.trim().is_empty() {
            continue;
        }
        out.push_str(indent);
        out.push_str(&line[common.min(line.len() - line.trim_start().len())..]);
    }
    out
}

enum Piece {
    Inline(String),
    Element(String),
    Space(String),
    Block(String),
}

fn text_piece(raw: &str, html: bool) -> Piece {
    let escape = |text: &str| match html {
        true => escape_html_text(text),
        false => escape_str_text(text),
    };
    if raw == " " {
        return Piece::Space(raw.to_string());
    }
    if !plain_space(raw) || raw.contains('\n') {
        return Piece::Inline(match html {
            true => html_expression(raw),
            false => format!("{{{}}}", js_string(raw)),
        });
    }
    Piece::Inline(escape(raw))
}

enum Entity {
    Char(char),
    Named,
}

fn entity(text: &str) -> Option<(usize, Entity)> {
    let end = text.find(';').filter(|at| *at <= 33)?;
    let name = &text[1..end];
    let number = |digits: &str, radix: u32| {
        (!digits.is_empty())
            .then(|| u32::from_str_radix(digits, radix).ok())
            .flatten()
            .and_then(char::from_u32)
    };
    let found = match name {
        "lt" => Entity::Char('<'),
        "gt" => Entity::Char('>'),
        "quot" => Entity::Char('"'),
        "apos" => Entity::Char('\''),
        "nbsp" => Entity::Char('\u{a0}'),
        "amp" => Entity::Char('&'),
        _ if name.starts_with("#x") || name.starts_with("#X") => {
            Entity::Char(number(&name[2..], 16)?)
        }
        _ if name.starts_with('#') => Entity::Char(number(&name[1..], 10)?),
        _ if !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric()) => Entity::Named,
        _ => return None,
    };
    Some((end + 1, found))
}

fn html_expression(raw: &str) -> String {
    let mut out = String::new();
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut String| {
        if !run.is_empty() {
            out.push_str(&format!("{{{}}}", js_string(run)));
            run.clear();
        }
    };
    let mut rest = raw;
    while let Some(at) = rest.find('&') {
        run.push_str(&rest[..at]);
        rest = &rest[at..];
        match entity(rest) {
            Some((length, Entity::Char(c))) => {
                run.push(c);
                rest = &rest[length..];
            }
            Some((length, Entity::Named)) => {
                flush(&mut run, &mut out);
                out.push_str(&rest[..length]);
                rest = &rest[length..];
            }
            None => {
                run.push('&');
                rest = &rest[1..];
            }
        }
    }
    run.push_str(rest);
    flush(&mut run, &mut out);
    out
}

fn pieces(children: &[Child]) -> Vec<Piece> {
    children
        .iter()
        .map(|child| match child {
            Child::Html(text) => text_piece(text, true),
            Child::Str(text) => text_piece(text, false),
            Child::Expr(text) => Piece::Inline(format!("{{{text}}}")),
            Child::Jsx(text) => match text.contains('\n') {
                true => Piece::Block(text.clone()),
                false => Piece::Element(text.clone()),
            },
        })
        .collect()
}

fn protect(line: &str) -> String {
    let lead = line.len() - line.trim_start_matches(' ').len();
    let trail = line.len() - line.trim_end_matches(' ').len();
    if line.trim().is_empty() {
        return format!("{{{}}}", js_string(line));
    }
    let mut out = String::new();
    if lead > 0 {
        out.push_str(&format!("{{{}}}", js_string(&line[..lead])));
    }
    out.push_str(&line[lead..line.len() - trail]);
    if trail > 0 {
        out.push_str(&format!("{{{}}}", js_string(&line[line.len() - trail..])));
    }
    out
}

pub fn children_lines(children: &[Child]) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    let mut after_element = false;
    let flush = |line: &mut String, lines: &mut Vec<String>| {
        if !line.is_empty() {
            lines.push(protect(line));
            line.clear();
        }
    };
    for piece in pieces(children) {
        match piece {
            Piece::Inline(text) => {
                if after_element && !text.starts_with(' ') {
                    flush(&mut line, &mut lines);
                }
                line.push_str(&text);
                after_element = false;
            }
            Piece::Space(text) => {
                line.push_str(&text);
                after_element = false;
            }
            Piece::Element(text) => {
                if !line.ends_with(' ') {
                    flush(&mut line, &mut lines);
                }
                line.push_str(&text);
                after_element = true;
            }
            Piece::Block(text) => {
                flush(&mut line, &mut lines);
                lines.push(text);
                after_element = true;
            }
        }
    }
    flush(&mut line, &mut lines);
    lines
}

fn inline_children(children: &[Child]) -> Option<String> {
    let mut out = String::new();
    let mut after_element = false;
    for piece in pieces(children) {
        match piece {
            Piece::Inline(text) | Piece::Space(text) => {
                out.push_str(&text);
                after_element = false;
            }
            Piece::Element(text) => {
                if after_element {
                    return None;
                }
                out.push_str(&text);
                after_element = true;
            }
            Piece::Block(_) => return None,
        }
    }
    (!out.contains('\n')).then_some(out)
}

pub fn element(name: &str, attrs: &[String], children: &[Child]) -> String {
    let flat = attrs.iter().all(|attr| !attr.contains('\n'));
    let joined: String = attrs.iter().map(|attr| format!(" {attr}")).collect();
    let open_line = flat && name.len() + joined.len() + 2 <= WIDTH;
    let open = match open_line {
        true => format!("<{name}{joined}"),
        false => {
            let mut text = format!("<{name}");
            for attr in attrs {
                text.push('\n');
                text.push_str(INDENT);
                text.push_str(&indent_rest(attr, INDENT));
            }
            text.push('\n');
            text
        }
    };
    if children.is_empty() {
        return match open_line {
            true => format!("{open} />"),
            false => format!("{open}/>"),
        };
    }
    if open_line {
        if let Some(inner) = inline_children(children) {
            let text = format!("{open}>{inner}</{name}>");
            if text.len() <= WIDTH {
                return text;
            }
        }
    }
    let mut out = format!("{open}>");
    for line in children_lines(children) {
        out.push('\n');
        out.push_str(INDENT);
        out.push_str(&indent_rest(&line, INDENT));
    }
    out.push_str(&format!("\n</{name}>"));
    out
}

pub fn fragment(children: &[Child]) -> String {
    if let Some(inner) = inline_children(children) {
        if inner.len() + 5 <= WIDTH {
            return format!("<>{inner}</>");
        }
    }
    let mut out = String::from("<>");
    for line in children_lines(children) {
        out.push('\n');
        out.push_str(INDENT);
        out.push_str(&indent_rest(&line, INDENT));
    }
    out.push_str("\n</>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_elements_stay_on_one_line() {
        let text = element(
            "p",
            &["class=\"a\"".to_string()],
            &[Child::Html("Count: ".into()), Child::Expr("count()".into())],
        );
        assert_eq!(text, "<p class=\"a\">Count: {count()}</p>");
    }

    #[test]
    fn edge_spaces_are_protected_when_children_break_lines() {
        let inner = element("b", &[], &[Child::Html("x".into())]);
        let block = format!("<div>\n  {inner}\n</div>");
        let text = element(
            "p",
            &[],
            &[
                Child::Html("first ".into()),
                Child::Jsx(block),
                Child::Html(" last".into()),
            ],
        );
        assert_eq!(
            text,
            "<p>\n  first{\" \"}\n  <div>\n    <b>x</b>\n  </div>\n  {\" \"}last\n</p>"
        );
    }

    #[test]
    fn empty_elements_self_close() {
        assert_eq!(
            element("input", &["value={v()}".into()], &[]),
            "<input value={v()} />"
        );
    }
}
