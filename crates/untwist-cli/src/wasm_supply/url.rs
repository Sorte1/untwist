use ureq::http::Uri;

pub fn has_scheme(text: &str) -> bool {
    let Some(end) = text.find(':') else {
        return false;
    };
    let mut chars = text[..end].chars();
    let starts = chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic());
    starts && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

pub fn fetchable(url: &str) -> Option<String> {
    let url = without_fragment(url);
    let uri: Uri = url.parse().ok()?;
    let scheme = uri.scheme_str()?;
    let known = scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https");
    let host = uri.host().is_some_and(|host| !host.is_empty());
    (known && host).then(|| url.to_string())
}

struct Base<'a> {
    scheme: &'a str,
    authority: &'a str,
    path: &'a str,
    query: &'a str,
}

fn split_query(text: &str) -> (&str, &str) {
    match text.find('?') {
        Some(at) => text.split_at(at),
        None => (text, ""),
    }
}

fn without_fragment(text: &str) -> &str {
    text.split_once('#').map_or(text, |(head, _)| head)
}

fn base_of(base: &str) -> Option<Base<'_>> {
    fetchable(base)?;
    let (scheme, rest) = without_fragment(base).split_once("://")?;
    let path_start = rest.find(['/', '?']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(path_start);
    let (path, query) = split_query(tail);
    Some(Base {
        scheme,
        authority,
        path,
        query,
    })
}

fn without_dot_segments(path: &str) -> String {
    let mut kept: Vec<&str> = Vec::new();
    let segments: Vec<&str> = path.split('/').skip(1).collect();
    let last = segments.len().saturating_sub(1);
    for (index, segment) in segments.iter().enumerate() {
        match *segment {
            "." | ".." => {
                if *segment == ".." {
                    kept.pop();
                }
                if index == last {
                    kept.push("");
                }
            }
            other => kept.push(other),
        }
    }
    format!("/{}", kept.join("/"))
}

fn merged(base_path: &str, reference_path: &str) -> String {
    let directory = base_path.rfind('/').map_or("/", |at| &base_path[..=at]);
    format!("{directory}{reference_path}")
}

pub fn joined(base: &str, reference: &str) -> Option<String> {
    let reference = without_fragment(reference);
    if has_scheme(reference) {
        return fetchable(reference);
    }
    let base = base_of(base)?;
    let origin = format!("{}://{}", base.scheme, base.authority);
    let (path, query) = split_query(reference);
    let target = if reference.starts_with("//") {
        format!("{}:{reference}", base.scheme)
    } else if path.starts_with('/') {
        format!("{origin}{}{query}", without_dot_segments(path))
    } else if path.is_empty() {
        let query = if query.is_empty() { base.query } else { query };
        let path = if base.path.is_empty() { "/" } else { base.path };
        format!("{origin}{path}{query}")
    } else {
        let path = without_dot_segments(&merged(base.path, path));
        format!("{origin}{path}{query}")
    };
    fetchable(&target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn join(base: &str, reference: &str) -> Option<String> {
        joined(base, reference)
    }

    #[test]
    fn a_base_without_a_path_takes_the_reference_under_its_root() {
        assert_eq!(
            join("https://x.dev", "pkg/a.wasm").as_deref(),
            Some("https://x.dev/pkg/a.wasm")
        );
        assert_eq!(
            join("https://x.dev", "/pkg/a.wasm").as_deref(),
            Some("https://x.dev/pkg/a.wasm")
        );
        assert_eq!(
            join("http://x.dev:8080", "./a.wasm").as_deref(),
            Some("http://x.dev:8080/a.wasm")
        );
    }

    #[test]
    fn a_base_with_a_trailing_slash_is_a_directory() {
        assert_eq!(
            join("https://x.dev/", "pkg/a.wasm").as_deref(),
            Some("https://x.dev/pkg/a.wasm")
        );
        assert_eq!(
            join("https://x.dev/app/", "pkg/a.wasm").as_deref(),
            Some("https://x.dev/app/pkg/a.wasm")
        );
    }

    #[test]
    fn a_base_with_a_path_loses_its_last_segment() {
        assert_eq!(
            join("https://x.dev/app/index.html", "pkg/a.wasm").as_deref(),
            Some("https://x.dev/app/pkg/a.wasm")
        );
        assert_eq!(
            join("https://x.dev/app/index.html?v=1#top", "a.wasm?h=2").as_deref(),
            Some("https://x.dev/app/a.wasm?h=2")
        );
        assert_eq!(
            join("https://x.dev/app", "a.wasm").as_deref(),
            Some("https://x.dev/a.wasm")
        );
    }

    #[test]
    fn dot_segments_are_resolved_and_never_climb_above_the_root() {
        assert_eq!(
            join("https://x.dev/a/b/c.js", "../d/./e.wasm").as_deref(),
            Some("https://x.dev/a/d/e.wasm")
        );
        assert_eq!(
            join("https://x.dev/a/c.js", "../../../e.wasm").as_deref(),
            Some("https://x.dev/e.wasm")
        );
        assert_eq!(
            join("https://x.dev/a/b/c.js", "/x/../y.wasm").as_deref(),
            Some("https://x.dev/y.wasm")
        );
    }

    #[test]
    fn a_rooted_reference_replaces_the_whole_path() {
        assert_eq!(
            join("https://x.dev/app/", "/static/a.wasm").as_deref(),
            Some("https://x.dev/static/a.wasm")
        );
    }

    #[test]
    fn a_reference_with_its_own_host_keeps_only_the_scheme_of_the_base() {
        assert_eq!(
            join("https://x.dev/app/", "//cdn.dev/a.wasm").as_deref(),
            Some("https://cdn.dev/a.wasm")
        );
        assert_eq!(
            join("https://x.dev/app/", "http://cdn.dev/a.wasm").as_deref(),
            Some("http://cdn.dev/a.wasm")
        );
    }

    #[test]
    fn an_empty_reference_is_the_base_without_its_fragment() {
        assert_eq!(
            join("https://x.dev/a.wasm?v=1#top", "").as_deref(),
            Some("https://x.dev/a.wasm?v=1")
        );
    }

    #[test]
    fn other_schemes_and_malformed_urls_are_not_fetchable() {
        assert_eq!(join("https://x.dev/", "file:///etc/a.wasm"), None);
        assert_eq!(join("https://x.dev/", "data:application/wasm,a.wasm"), None);
        assert_eq!(join("ftp://x.dev/", "a.wasm"), None);
        assert_eq!(join("pkg/", "a.wasm"), None);
        assert_eq!(join("https://x.dev/", "a b.wasm"), None);
        assert_eq!(fetchable("http://x.dev/\u{1b}[31m/a.wasm"), None);
        assert_eq!(fetchable("http:///a.wasm"), None);
        assert_eq!(fetchable("x.dev/a.wasm"), None);
    }
}
