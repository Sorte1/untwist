use untwist_eval::decode::Decoded;
use untwist_eval::value::Value;

const SOURCES: [(&str, &str); 6] = [
    ("fold.rs", include_str!("../src/fold.rs")),
    (
        "fold/budget_host.rs",
        include_str!("../src/fold/budget_host.rs"),
    ),
    ("decode/mod.rs", include_str!("../src/decode/mod.rs")),
    (
        "decode/checksum.rs",
        include_str!("../src/decode/checksum.rs"),
    ),
    (
        "decode/dead_terms.rs",
        include_str!("../src/decode/dead_terms.rs"),
    ),
    (
        "decode/entries.rs",
        include_str!("../src/decode/entries.rs"),
    ),
];

const IN_REALM_HANDLE: &str = "impl<'realm> Session<'realm>";

const VALUE_BEARING: [&str; 6] = ["Value", "Abrupt", "Eval", "Realm", "FrameRef", "ObjRef"];

#[derive(Default)]
struct Surface {
    returns: Vec<(String, String)>,
    fields: Vec<String>,
    handles: usize,
}

fn bears_a_value(text: &str) -> bool {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .any(|token| VALUE_BEARING.contains(&token))
}

fn is_public(header: &str) -> bool {
    header.starts_with("pub ") && !header.starts_with("pub(")
}

fn last_line(text: &str) -> &str {
    text.lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty() && !line.starts_with("#["))
        .unwrap_or("")
}

fn declared_name(header: &str, kind: &str) -> Option<String> {
    let rest = header.strip_prefix("pub ")?.strip_prefix(kind)?;
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

fn public_types(sources: &[(&str, &str)]) -> Vec<String> {
    let mut names = Vec::new();
    for (_, source) in sources {
        for line in source.lines().map(str::trim) {
            for kind in ["struct ", "enum ", "trait "] {
                if let Some(name) = declared_name(line, kind) {
                    names.push(name);
                }
            }
        }
    }
    names
}

fn implemented_type(header: &str) -> Option<&str> {
    let (_, tail) = header.split_once(" for ")?;
    tail.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .find(|token| !token.is_empty())
}

fn scanned_fn(header: &str, enclosing: &str, public: &[String]) -> bool {
    if !header.contains("fn ") {
        return false;
    }
    if is_public(header) {
        return true;
    }
    if enclosing.starts_with("pub trait") {
        return true;
    }
    enclosing.starts_with("impl")
        && implemented_type(enclosing).is_some_and(|name| public.iter().any(|known| known == name))
}

fn public_items(body: &str, every: bool) -> Vec<String> {
    let mut items = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for c in body.chars() {
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth -= 1,
            _ => {}
        }
        if c == ',' && depth == 0 {
            items.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    items.push(current);
    items
        .into_iter()
        .map(|item| last_line(&item).to_string())
        .filter(|item| !item.is_empty() && (every || is_public(item)))
        .collect()
}

fn scan(file: &str, source: &str, public: &[String], surface: &mut Surface) {
    let mut headers: Vec<(String, usize)> = Vec::new();
    let mut start = 0;
    for (at, byte) in source.bytes().enumerate() {
        match byte {
            b'{' => {
                let header = last_statement(&source[start..at]);
                let enclosing = headers.last().map(|(h, _)| h.clone()).unwrap_or_default();
                if header == IN_REALM_HANDLE {
                    surface.handles += 1;
                }
                if scanned_fn(&header, &enclosing, public) && enclosing != IN_REALM_HANDLE {
                    surface.returns.push((file.to_string(), header.clone()));
                }
                headers.push((header, at + 1));
                start = at + 1;
            }
            b'}' => {
                if let Some((header, opened)) = headers.pop() {
                    record_body(file, &header, &source[opened..at], surface);
                }
                start = at + 1;
            }
            b';' => {
                let header = last_statement(&source[start..at]);
                let enclosing = headers.last().map(|(h, _)| h.clone()).unwrap_or_default();
                if scanned_fn(&header, &enclosing, public) && enclosing != IN_REALM_HANDLE {
                    surface.returns.push((file.to_string(), header.clone()));
                }
                if let Some(fields) = tuple_fields(&header) {
                    record_fields(file, &header, &fields, false, surface);
                }
                start = at + 1;
            }
            _ => {}
        }
    }
}

fn last_statement(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("#[") && !line.starts_with("//"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn tuple_fields(header: &str) -> Option<String> {
    declared_name(header, "struct ")?;
    let open = header.find('(')?;
    let close = header.rfind(')')?;
    Some(header[open + 1..close].to_string())
}

fn record_body(file: &str, header: &str, body: &str, surface: &mut Surface) {
    if declared_name(header, "struct ").is_some() {
        record_fields(file, header, body, false, surface);
    }
    if declared_name(header, "enum ").is_some() {
        record_fields(file, header, body, true, surface);
    }
}

fn record_fields(file: &str, header: &str, body: &str, every: bool, surface: &mut Surface) {
    for item in public_items(body, every) {
        if bears_a_value(&item) {
            surface.fields.push(format!("{file}: {header} :: {item}"));
        }
    }
}

fn returned(signature: &str) -> &str {
    signature
        .split_once("->")
        .map_or("", |(_, tail)| tail.split(" where ").next().unwrap_or(tail))
}

fn surface(sources: &[(&str, &str)]) -> Surface {
    let public = public_types(sources);
    let mut surface = Surface::default();
    for (file, source) in sources {
        scan(file, source, &public, &mut surface);
    }
    surface
}

#[test]
fn no_public_function_or_field_of_fold_or_decode_bears_a_value() {
    let surface = surface(&SOURCES);
    assert_eq!(
        surface.handles, 1,
        "the realm handle's accessors sit in exactly one impl block"
    );
    assert!(
        surface.returns.len() > 20,
        "the scan found only {} signatures",
        surface.returns.len()
    );
    let offending: Vec<String> = surface
        .returns
        .iter()
        .filter(|(_, signature)| bears_a_value(returned(signature)))
        .map(|(file, signature)| format!("{file}: {signature}"))
        .chain(surface.fields.iter().cloned())
        .collect();
    assert!(offending.is_empty(), "{offending:#?}");
}

#[test]
fn a_decoded_result_is_no_larger_than_a_value() {
    assert_eq!(
        std::mem::size_of::<Option<Decoded>>(),
        std::mem::size_of::<Option<Value<'static>>>()
    );
}
