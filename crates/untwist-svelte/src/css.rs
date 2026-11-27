use std::cell::RefCell;

use crate::scope::El;

struct Sheet<'e> {
    hash: &'e str,
    elements: &'e [El],
    locals: RefCell<Vec<Vec<String>>>,
    reason: RefCell<Option<&'static str>>,
}

const PARSE: &str = "a style sheet that does not parse";
const GLOBAL: &str = "a style selector whose :global parts cannot be told apart";
const COVER: &str = "a scoped element that the restored style would not match";

pub fn restore(
    code: &str,
    hash: &str,
    elements: &[El],
    scoped: &[El],
) -> Result<String, &'static str> {
    let code = strip_unused(code).ok_or(PARSE)?;
    let code = global_frames(&code, hash);
    let code = code.replace(&format!("{hash}-"), "");
    let sheet = Sheet {
        hash,
        elements,
        locals: RefCell::default(),
        reason: RefCell::default(),
    };
    let mut out = String::new();
    if block(&code, &sheet, 1, &mut out).is_none() {
        return Err(sheet.reason.take().unwrap_or(PARSE));
    }
    if out.contains(hash) {
        return Err(PARSE);
    }
    match sheet.covers(scoped) {
        true => Ok(out),
        false => Err(COVER),
    }
}

fn global_frames(code: &str, hash: &str) -> String {
    const HEAD: &str = "@keyframes ";
    let scoped = format!("{hash}-");
    let mut out = String::new();
    let mut rest = code;
    while let Some(at) = rest.find(HEAD) {
        let name_at = at + HEAD.len();
        out.push_str(&rest[..name_at]);
        let name = rest[name_at..].trim_start();
        if !name.starts_with(&scoped) && !name.starts_with('{') {
            out.push_str(&rest[name_at..rest.len() - name.len()]);
            out.push_str("-global-");
            rest = name;
            continue;
        }
        rest = &rest[name_at..];
    }
    out.push_str(rest);
    out
}

fn strip_unused(code: &str) -> Option<String> {
    let mut out = String::new();
    let mut rest = code;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        let end = rest[start..].find("*/")? + start + 2;
        rest = &rest[end..];
    }
    out.push_str(rest);
    Some(out)
}

fn split_top(text: &str, at: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut start = 0;
    for (index, c) in text.char_indices() {
        match (quote, c) {
            (Some(q), _) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '(' | '[') => depth += 1,
            (None, ')' | ']') => depth -= 1,
            (None, _) if c == at && depth == 0 => {
                parts.push(&text[start..index]);
                start = index + c.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
}

fn matching(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    for (index, c) in text[open..].char_indices() {
        match (quote, c) {
            (Some(q), _) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '{') => depth += 1,
            (None, '}') => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + index);
                }
            }
            _ => {}
        }
    }
    None
}

fn block(text: &str, sheet: &Sheet<'_>, depth: usize, out: &mut String) -> Option<()> {
    let pad = "\t".repeat(depth);
    let mut rest = text.trim();
    while !rest.is_empty() {
        let open = rest.find('{')?;
        let close = matching(rest, open)?;
        let head = rest[..open].trim();
        let body = &rest[open + 1..close];
        if !out.is_empty() {
            out.push('\n');
        }
        if head.starts_with("@media")
            || head.starts_with("@supports")
            || head.starts_with("@container")
            || head.starts_with("@layer")
        {
            out.push_str(&format!("{pad}{head} {{\n"));
            let mut inner = String::new();
            block(body, sheet, depth + 1, &mut inner)?;
            out.push_str(&inner);
            out.push_str(&format!("\n{pad}}}"));
        } else if head.starts_with("@keyframes")
            || head.starts_with("@font-face")
            || head.starts_with("@page")
        {
            out.push_str(&format!("{pad}{head} {{\n"));
            match head.starts_with("@keyframes") {
                true => {
                    let mut inner = String::new();
                    frames(body, depth + 1, &mut inner)?;
                    out.push_str(&inner);
                }
                false => declarations(body, depth + 1, out),
            }
            out.push_str(&format!("\n{pad}}}"));
        } else if head.starts_with('@') || body.contains('{') {
            return None;
        } else {
            let selectors: Option<Vec<String>> = split_top(head, ',')
                .into_iter()
                .map(|selector| sheet.complex(selector.trim()))
                .collect();
            out.push_str(&format!(
                "{pad}{} {{\n",
                selectors?.join(&format!(",\n{pad}"))
            ));
            declarations(body, depth + 1, out);
            out.push_str(&format!("\n{pad}}}"));
        }
        rest = rest[close + 1..].trim();
    }
    Some(())
}

fn frames(text: &str, depth: usize, out: &mut String) -> Option<()> {
    let pad = "\t".repeat(depth);
    let mut rest = text.trim();
    while !rest.is_empty() {
        let open = rest.find('{')?;
        let close = matching(rest, open)?;
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!("{pad}{} {{\n", rest[..open].trim()));
        declarations(&rest[open + 1..close], depth + 1, out);
        out.push_str(&format!("\n{pad}}}"));
        rest = rest[close + 1..].trim();
    }
    Some(())
}

fn declarations(text: &str, depth: usize, out: &mut String) {
    let pad = "\t".repeat(depth);
    let lines: Vec<String> = split_top(text, ';')
        .into_iter()
        .map(str::trim)
        .filter(|declaration| !declaration.is_empty())
        .map(|declaration| match declaration.split_once(':') {
            Some((name, value)) => format!("{pad}{}: {};", name.trim(), value.trim()),
            None => format!("{pad}{declaration};"),
        })
        .collect();
    out.push_str(&lines.join("\n"));
}

impl Sheet<'_> {
    fn complex(&self, selector: &str) -> Option<String> {
        let mut out = String::new();
        let mut compound = String::new();
        let mut depth = 0i32;
        let mut pending: Option<String> = None;
        let flush =
            |compound: &mut String, out: &mut String, pending: &mut Option<String>| -> Option<()> {
                if compound.is_empty() {
                    return Some(());
                }
                if let Some(combinator) = pending.take() {
                    out.push_str(&combinator);
                }
                out.push_str(&self.scoped(compound)?);
                compound.clear();
                Some(())
            };
        for c in selector.chars() {
            match c {
                '(' | '[' => {
                    depth += 1;
                    compound.push(c);
                }
                ')' | ']' => {
                    depth -= 1;
                    compound.push(c);
                }
                '>' | '+' | '~' if depth == 0 => {
                    flush(&mut compound, &mut out, &mut pending)?;
                    pending = Some(format!(" {c} "));
                }
                c if c.is_whitespace() && depth == 0 => {
                    flush(&mut compound, &mut out, &mut pending)?;
                    if pending.is_none() && !out.is_empty() {
                        pending = Some(" ".to_string());
                    }
                }
                _ => compound.push(c),
            }
        }
        flush(&mut compound, &mut out, &mut pending)?;
        let out = out.trim_start().to_string();
        (!out.is_empty()).then_some(out)
    }

    fn is_mark(&self, part: &str) -> bool {
        part.strip_prefix('.') == Some(self.hash)
            || part
                .strip_prefix(":where(.")
                .and_then(|rest| rest.strip_suffix(')'))
                == Some(self.hash)
    }

    fn scoped(&self, compound: &str) -> Option<String> {
        let parts = parts(compound);
        let Some(at) = parts.iter().position(|part| self.is_mark(part)) else {
            return self.unscoped(&parts);
        };
        let after: Vec<&str> = parts[at + 1..]
            .iter()
            .copied()
            .filter(|part| !self.is_mark(part))
            .collect();
        let twice = after.len() + at + 1 < parts.len();
        let head = self.before(&parts[..at])?;
        let tail = self.after(&after, twice)?;
        Some(format!("{head}{tail}"))
    }

    fn unscoped(&self, parts: &[&str]) -> Option<String> {
        if !parts.iter().any(|part| part.contains(self.hash)) {
            return Some(format!(":global({})", parts.concat()));
        }
        let mut out = String::new();
        let mut group = String::new();
        for part in parts {
            match self.functional(part)? {
                Some(text) => {
                    push_global(&mut out, &mut group, true);
                    out.push_str(&text);
                }
                None => group.push_str(part),
            }
        }
        push_global(&mut out, &mut group, true);
        Some(out)
    }

    fn before(&self, parts: &[&str]) -> Option<String> {
        if parts.len() < 2 {
            let mut text = String::new();
            for part in parts {
                text.push_str(&self.functional(part)?.unwrap_or_else(|| part.to_string()));
            }
            self.locals
                .borrow_mut()
                .push(parts.iter().map(|part| part.to_string()).collect());
            return Some(if text.is_empty() {
                "*".to_string()
            } else {
                text
            });
        }
        if parts.iter().any(|part| part.starts_with(':')) {
            return self.refuse();
        }
        let carried: Vec<bool> = parts
            .iter()
            .map(|part| self.elements.iter().any(|el| el.carries(part)))
            .collect();
        let global = carried.iter().take_while(|carried| !**carried).count();
        if global == parts.len() || carried[global..].iter().any(|carried| !carried) {
            return self.refuse();
        }
        self.locals.borrow_mut().push(
            parts[global..]
                .iter()
                .map(|part| part.to_string())
                .collect(),
        );
        Some(match global {
            0 => parts.concat(),
            _ => format!(
                ":global({}){}",
                parts[..global].concat(),
                parts[global..].concat()
            ),
        })
    }

    fn after(&self, parts: &[&str], twice: bool) -> Option<String> {
        let mut out = String::new();
        let mut group = String::new();
        for part in parts {
            match self.functional(part)? {
                Some(text) => {
                    let wrap = !twice && has_simple(&group);
                    push_global(&mut out, &mut group, wrap);
                    out.push_str(&text);
                }
                None => group.push_str(part),
            }
        }
        let wrap = !twice && has_simple(&group);
        push_global(&mut out, &mut group, wrap);
        Some(out)
    }

    fn functional(&self, part: &str) -> Option<Option<String>> {
        let Some(name) = [":is(", ":where(", ":has("]
            .into_iter()
            .find(|name| part.starts_with(name))
        else {
            return Some(None);
        };
        if self.is_mark(part) || !part.ends_with(')') {
            return Some(None);
        }
        let inner = &part[name.len()..part.len() - 1];
        let args: Option<Vec<String>> = split_top(inner, ',')
            .into_iter()
            .map(str::trim)
            .filter(|arg| !arg.is_empty())
            .map(|arg| self.complex(arg))
            .collect();
        let args = args?;
        if args.is_empty() {
            return self.refuse();
        }
        Some(Some(format!("{name}{})", args.join(", "))))
    }

    fn refuse<T>(&self) -> Option<T> {
        self.reason.replace(Some(GLOBAL));
        None
    }

    fn covers(&self, scoped: &[El]) -> bool {
        let locals = self.locals.borrow();
        scoped.iter().all(|el| {
            locals
                .iter()
                .any(|parts| parts.iter().all(|part| el.carries(part)))
        })
    }
}

fn push_global(out: &mut String, group: &mut String, wrap: bool) {
    if group.is_empty() {
        return;
    }
    match wrap {
        true => out.push_str(&format!(":global({group})")),
        false => out.push_str(group),
    }
    group.clear();
}

fn has_simple(text: &str) -> bool {
    let mut depth = 0i32;
    for c in text.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            '.' | '#' | '[' if depth == 0 => return true,
            _ => {}
        }
    }
    false
}

fn parts(compound: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut previous = '\0';
    for (index, c) in compound.char_indices() {
        let split = match (escaped, quote, c) {
            (true, _, _) => {
                escaped = false;
                false
            }
            (_, _, '\\') => {
                escaped = true;
                false
            }
            (_, Some(q), _) => {
                if c == q {
                    quote = None;
                }
                false
            }
            (_, None, '"' | '\'') => {
                quote = Some(c);
                false
            }
            (_, None, '(' | '[') => {
                depth += 1;
                c == '[' && depth == 1
            }
            (_, None, ')' | ']') => {
                depth -= 1;
                false
            }
            (_, None, '.' | '#') => depth == 0,
            (_, None, ':') => depth == 0 && previous != ':',
            _ => false,
        };
        if split && index > start {
            out.push(&compound[start..index]);
            start = index;
        }
        previous = c;
    }
    if start < compound.len() {
        out.push(&compound[start..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::scope::El;

    fn restore(code: &str, hash: &str) -> Option<String> {
        super::restore(code, hash, &[], &[]).ok()
    }

    fn el(name: &str, attrs: &[&str]) -> El {
        let attrs: Vec<String> = attrs.iter().map(|attr| attr.to_string()).collect();
        El::of(name, &attrs).unwrap()
    }

    #[test]
    fn removes_the_hash_and_wraps_unscoped_compounds() {
        let css = restore(
            ".a.svelte-x .b.svelte-x {color:red;}body div.svelte-x {margin:0 4px;}",
            "svelte-x",
        )
        .unwrap();
        assert_eq!(
            css,
            "\t.a .b {\n\t\tcolor: red;\n\t}\n\t:global(body) div {\n\t\tmargin: 0 4px;\n\t}"
        );
    }

    #[test]
    fn keeps_keyframes_and_media_blocks() {
        let css = restore("@media (min-width: 10px) {p.svelte-x {color:red;}}@keyframes svelte-x-spin {from {opacity:0;}to {opacity:1;}}p.svelte-x {animation:svelte-x-spin 1s;}", "svelte-x").unwrap();
        assert!(css.contains("@keyframes spin {"), "{css}");
        assert!(!css.contains("-global-"), "{css}");
        assert!(css.contains("animation: spin 1s;"), "{css}");
        assert!(
            css.contains("\t@media (min-width: 10px) {\n\t\tp {"),
            "{css}"
        );
    }

    #[test]
    fn a_class_after_the_hash_came_from_global() {
        let css = restore(
            "span.svelte-x.dyn {color:teal;}p.svelte-x.x:hover {left:0}p.svelte-x:hover {top:0}",
            "svelte-x",
        )
        .unwrap();
        assert!(css.contains("span:global(.dyn) {"), "{css}");
        assert!(css.contains("p:global(.x:hover) {"), "{css}");
        assert!(css.contains("p:hover {"), "{css}");
    }

    #[test]
    fn unscoped_keyframes_are_global() {
        let css = restore(
            "@keyframes pulse { from { opacity: 0 } }span.svelte-x { animation: pulse 2s;}",
            "svelte-x",
        )
        .unwrap();
        assert!(css.contains("@keyframes -global-pulse {"), "{css}");
        assert!(css.contains("animation: pulse 2s;"), "{css}");
    }

    #[test]
    fn nested_rules_are_refused() {
        assert!(restore(
            ".box.svelte-x {& p:where(.svelte-x) {color:green;}}",
            "svelte-x"
        )
        .is_none());
        assert!(restore(".box.svelte-x {.q {right:1px;}}", "svelte-x").is_none());
    }

    #[test]
    fn a_leading_part_no_element_carries_came_from_global() {
        let elements = [el("div", &["class=\"b\""])];
        let css = super::restore(
            ".dyn.b.svelte-x {color:teal;}",
            "svelte-x",
            &elements,
            &elements,
        )
        .unwrap();
        assert!(css.contains(":global(.dyn).b {"), "{css}");
        let css = super::restore(
            "#app.b.svelte-x {color:teal;}",
            "svelte-x",
            &elements,
            &elements,
        )
        .unwrap();
        assert!(css.contains(":global(#app).b {"), "{css}");
        let both = [el("div", &["class=\"dyn b\""])];
        let css =
            super::restore(".dyn.b.svelte-x {color:teal;}", "svelte-x", &both, &both).unwrap();
        assert!(css.contains("\t.dyn.b {"), "{css}");
    }

    #[test]
    fn a_scoped_element_the_restored_selectors_miss_is_refused() {
        let elements = [
            el("div", &["class=\"dyn b\""]),
            el("span", &["class=\"b\""]),
        ];
        assert!(super::restore(
            ".dyn.b.svelte-x {color:teal;}",
            "svelte-x",
            &elements,
            &elements
        )
        .is_err());
    }

    #[test]
    fn a_part_no_element_carries_after_a_carried_one_is_refused() {
        let elements = [el("div", &["class=\"b\""])];
        assert!(
            super::restore(".b.dyn.svelte-x {color:teal;}", "svelte-x", &elements, &[]).is_err()
        );
    }

    #[test]
    fn unscoped_compounds_inside_has_is_and_where_came_from_global() {
        let css = restore(
            "p.svelte-x:has(.x) {top:0}p.svelte-x:has(> .x) {top:1px}p.svelte-x:where(.x) {top:2px}span.svelte-x:is(.z /* (unused) .w*/) {top:3px}",
            "svelte-x",
        )
        .unwrap();
        assert!(css.contains("p:has(:global(.x)) {"), "{css}");
        assert!(css.contains("p:has(> :global(.x)) {"), "{css}");
        assert!(css.contains("p:where(:global(.x)) {"), "{css}");
        assert!(css.contains("span:is(:global(.z)) {"), "{css}");
    }

    #[test]
    fn scoped_compounds_inside_has_and_is_stay_scoped() {
        let css = restore(
            "p.svelte-x:has(> .x:where(.svelte-x), b:where(.svelte-x)) {top:0}:is(p:where(.svelte-x)) span.svelte-x {top:1px}i.svelte-x.zz:is(.z:where(.svelte-x)) {top:2px}p.svelte-x .x:has(b:where(.svelte-x)) {top:3px}",
            "svelte-x",
        )
        .unwrap();
        assert!(css.contains("p:has(> .x, b) {"), "{css}");
        assert!(css.contains(":is(p) span {"), "{css}");
        assert!(css.contains("i:global(.zz):is(.z) {"), "{css}");
        assert!(css.contains("p :global(.x):has(b) {"), "{css}");
    }
}
