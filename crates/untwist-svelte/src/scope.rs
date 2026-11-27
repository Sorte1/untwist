use crate::markup::M;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct El {
    tag: String,
    classes: Vec<String>,
    ids: Vec<String>,
    attrs: Vec<String>,
    any: bool,
    any_class: bool,
    any_id: bool,
}

impl El {
    pub fn of(name: &str, attrs: &[String]) -> Option<Self> {
        if name.starts_with(|c: char| c.is_ascii_uppercase()) || name.contains('.') {
            return None;
        }
        if name.starts_with("svelte:") {
            return (name == "svelte:element").then(|| El {
                any: true,
                ..El::default()
            });
        }
        let mut el = El {
            tag: name.to_ascii_lowercase(),
            ..El::default()
        };
        for attr in attrs {
            el.read(attr);
        }
        Some(el)
    }

    fn read(&mut self, attr: &str) {
        if attr.starts_with("{...") {
            self.any = true;
            return;
        }
        if let Some(inner) = attr
            .strip_prefix('{')
            .and_then(|rest| rest.strip_suffix('}'))
        {
            self.dynamic(inner.trim());
            return;
        }
        let (name, value) = match attr.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (attr, None),
        };
        if let Some(class) = name.strip_prefix("class:") {
            self.classes.push(unescape(class));
            return;
        }
        if name.contains(':') {
            return;
        }
        let fixed = value
            .and_then(|value| value.strip_prefix('"'))
            .and_then(|value| value.strip_suffix('"'))
            .filter(|value| !value.contains(['{', '&']));
        match (name, value.is_some(), fixed) {
            ("class", _, Some(value)) => {
                self.classes.extend(value.split_whitespace().map(unescape))
            }
            ("id", _, Some(value)) => self.ids.push(value.trim().to_string()),
            (_, true, None) => return self.dynamic(name),
            _ => {}
        }
        self.attrs.push(name.to_ascii_lowercase());
    }

    fn dynamic(&mut self, name: &str) {
        match name {
            "class" => self.any_class = true,
            "id" => self.any_id = true,
            _ => {}
        }
        self.attrs.push(name.to_ascii_lowercase());
    }

    pub fn carries(&self, part: &str) -> bool {
        if self.any {
            return true;
        }
        match part.chars().next() {
            Some('.') => self.any_class || self.classes.contains(&unescape(&part[1..])),
            Some('#') => self.any_id || self.ids.contains(&unescape(&part[1..])),
            Some('[') => {
                let name = part[1..]
                    .split(['=', '~', '|', '^', '$', '*', ']'])
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_ascii_lowercase();
                self.attrs.contains(&name)
            }
            Some(':' | '*') | None => true,
            Some(_) => self.tag.eq_ignore_ascii_case(&unescape(part)),
        }
    }
}

fn unescape(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.extend(chars.next()),
            c => out.push(c),
        }
    }
    out
}

pub fn elements(nodes: &[M]) -> Vec<El> {
    let mut out = Vec::new();
    collect(nodes, &mut out);
    out
}

fn collect(nodes: &[M], out: &mut Vec<El>) {
    for node in nodes {
        match node {
            M::Element {
                name,
                attrs,
                children,
                ..
            } => {
                out.extend(El::of(name, attrs));
                collect(children, out);
            }
            M::Block { sections, .. } => {
                for (_, children) in sections {
                    collect(children, out);
                }
            }
            M::Text(_) | M::Tag(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::El;

    fn el(name: &str, attrs: &[&str]) -> El {
        let attrs: Vec<String> = attrs.iter().map(|attr| attr.to_string()).collect();
        El::of(name, &attrs).unwrap()
    }

    #[test]
    fn static_classes_and_directives_are_carried() {
        let div = el("div", &["class=\"b c\"", "class:on={x}", "title=\"t\""]);
        assert!(div.carries(".b") && div.carries(".on") && div.carries("div"));
        assert!(div.carries("[title]") && div.carries(":hover"));
        assert!(!div.carries(".dyn") && !div.carries("#app") && !div.carries("p"));
    }

    #[test]
    fn dynamic_classes_and_spreads_may_carry_anything() {
        assert!(el("p", &["class={x}"]).carries(".dyn"));
        assert!(!el("p", &["class={x}"]).carries("#id"));
        assert!(el("p", &["{...rest}"]).carries("#id"));
        assert!(El::of("svelte:element", &[]).unwrap().carries("section"));
        assert!(El::of("Child", &[]).is_none());
    }
}
