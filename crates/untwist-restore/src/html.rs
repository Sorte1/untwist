#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Element(Element),
    Text(String),
    Comment(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub attrs: Vec<(String, Option<String>)>,
    pub children: Vec<Node>,
}

const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

const RAW: &[&str] = &["script", "style", "textarea", "title"];

pub fn is_void(name: &str) -> bool {
    VOID.contains(&name)
}

pub fn parse(text: &str) -> Option<Vec<Node>> {
    let mut parser = Parser {
        text,
        at: 0,
        open_ended: false,
    };
    let nodes = parser.nodes(None)?;
    (parser.at == text.len()).then_some(nodes)
}

pub fn parse_open_ended(text: &str) -> Option<Vec<Node>> {
    let mut parser = Parser {
        text,
        at: 0,
        open_ended: true,
    };
    let nodes = parser.nodes(None)?;
    (parser.at == text.len()).then_some(nodes)
}

struct Parser<'t> {
    text: &'t str,
    at: usize,
    open_ended: bool,
}

impl<'t> Parser<'t> {
    fn rest(&self) -> &'t str {
        &self.text[self.at..]
    }

    fn nodes(&mut self, closing: Option<&str>) -> Option<Vec<Node>> {
        let mut nodes = Vec::new();
        loop {
            let rest = self.rest();
            if rest.is_empty() {
                return (closing.is_none() || self.open_ended).then_some(nodes);
            }
            if let Some(after) = rest.strip_prefix("</") {
                let name = closing?;
                let end = after.find('>')?;
                if after[..end].trim() != name {
                    return None;
                }
                self.at += 2 + end + 1;
                return Some(nodes);
            }
            if rest.starts_with("<!>") {
                self.at += 3;
                nodes.push(Node::Comment(String::new()));
            } else if let Some(after) = rest.strip_prefix("<!--") {
                let end = after.find("-->")?;
                nodes.push(Node::Comment(after[..end].to_string()));
                self.at += 4 + end + 3;
            } else if rest.starts_with('<') {
                nodes.push(Node::Element(self.element()?));
            } else {
                let end = rest.find('<').unwrap_or(rest.len());
                nodes.push(Node::Text(rest[..end].to_string()));
                self.at += end;
            }
        }
    }

    fn name(&mut self) -> Option<String> {
        let rest = self.rest();
        let end = rest
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/' || c == '=')
            .unwrap_or(rest.len());
        (end > 0).then(|| {
            self.at += end;
            rest[..end].to_string()
        })
    }

    fn skip_space(&mut self) {
        let rest = self.rest();
        self.at += rest.len() - rest.trim_start().len();
    }

    fn element(&mut self) -> Option<Element> {
        self.at += 1;
        let name = self.name()?;
        let mut attrs = Vec::new();
        loop {
            self.skip_space();
            let rest = self.rest();
            if rest.starts_with("/>") {
                self.at += 2;
                return Some(Element {
                    name,
                    attrs,
                    children: Vec::new(),
                });
            }
            if rest.starts_with('>') {
                self.at += 1;
                break;
            }
            let attr = self.name()?;
            self.skip_space();
            let value = match self.rest().strip_prefix('=') {
                Some(_) => {
                    self.at += 1;
                    self.skip_space();
                    Some(self.value()?)
                }
                None => None,
            };
            attrs.push((attr, value));
        }
        if is_void(&name) {
            return Some(Element {
                name,
                attrs,
                children: Vec::new(),
            });
        }
        if RAW.contains(&name.as_str()) {
            let rest = self.rest();
            let end = match rest.find(&format!("</{name}")) {
                Some(end) => end,
                None if self.open_ended => rest.len(),
                None => return None,
            };
            let text = rest[..end].to_string();
            self.at += end;
            let closed = self.nodes(Some(&name))?;
            let children = std::iter::once(Node::Text(text))
                .filter(|node| node != &Node::Text(String::new()))
                .chain(closed)
                .collect();
            return Some(Element {
                name,
                attrs,
                children,
            });
        }
        let children = self.nodes(Some(&name))?;
        Some(Element {
            name,
            attrs,
            children,
        })
    }

    fn value(&mut self) -> Option<String> {
        let rest = self.rest();
        let quote = rest.chars().next()?;
        if quote == '"' || quote == '\'' {
            let end = rest[1..].find(quote)?;
            self.at += end + 2;
            return Some(rest[1..=end].to_string());
        }
        let end = rest
            .find(|c: char| c.is_whitespace() || c == '>')
            .unwrap_or(rest.len());
        self.at += end;
        Some(rest[..end].to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fragments_with_anchors_and_void_elements() {
        let nodes =
            parse(r#"<h2 class="a b"> </h2> <!> <input type="checkbox"/><img src="x">"#).unwrap();
        assert_eq!(nodes.len(), 6);
        assert_eq!(nodes[2], Node::Comment(String::new()));
        let Node::Element(input) = &nodes[4] else {
            panic!()
        };
        assert_eq!(
            input.attrs,
            vec![("type".to_string(), Some("checkbox".to_string()))]
        );
    }

    #[test]
    fn open_ended_parse_closes_trailing_elements() {
        let nodes = parse_open_ended("<div><p>Count: <!> / </p><button>+1").unwrap();
        let Node::Element(div) = &nodes[0] else {
            panic!()
        };
        assert_eq!(div.children.len(), 2);
        assert!(parse("<div><button>+1").is_none());
    }

    #[test]
    fn refuses_unbalanced_markup() {
        assert!(parse("<p><b></p>").is_none());
    }
}
