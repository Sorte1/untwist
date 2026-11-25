pub fn restore(code: &str, hash: &str) -> Option<String> {
    let code = strip_unused(code)?;
    let code = code.replace(&format!("{hash}-"), "");
    let mut out = String::new();
    block(&code, hash, 1, &mut out)?;
    Some(out)
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

fn block(text: &str, hash: &str, depth: usize, out: &mut String) -> Option<()> {
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
            block(body, hash, depth + 1, &mut inner)?;
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
        } else if head.starts_with('@') {
            return None;
        } else {
            let selectors: Option<Vec<String>> = split_top(head, ',')
                .into_iter()
                .map(|selector| complex(selector.trim(), hash))
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

fn complex(selector: &str, hash: &str) -> Option<String> {
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
            out.push_str(&scoped(compound, hash)?);
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
    (!out.is_empty()).then_some(out)
}

fn scoped(compound: &str, hash: &str) -> Option<String> {
    let class = format!(".{hash}");
    let wrapped = format!(":where({class})");
    if compound.contains(&wrapped) || compound.contains(&class) {
        let bare = compound.replace(&wrapped, "").replace(&class, "");
        return Some(if bare.is_empty() {
            "*".to_string()
        } else {
            bare
        });
    }
    Some(format!(":global({compound})"))
}

#[cfg(test)]
mod tests {
    use super::restore;

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
        assert!(css.contains("animation: spin 1s;"), "{css}");
        assert!(
            css.contains("\t@media (min-width: 10px) {\n\t\tp {"),
            "{css}"
        );
    }
}
