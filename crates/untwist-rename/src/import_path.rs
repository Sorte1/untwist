use crate::generate::to_identifier;

const EXTENSIONS: [&str; 7] = [".js", ".mjs", ".cjs", ".jsx", ".ts", ".tsx", ".json"];

const NOISE: [&str; 8] = ["index", "dist", "build", "src", "es", "esm", "cjs", "umd"];

pub fn import_base(source: &str) -> Option<String> {
    let path = source.split(['?', '#']).next().unwrap_or(source);
    let mut segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != "." && *segment != "..")
        .collect();
    let last = segments.pop()?;
    segments.push(strip_extension(last));
    let trimmed = drop_trailing_noise(&segments);
    let segments = if trimmed.is_empty() {
        segments.as_slice()
    } else {
        trimmed
    };
    let tail = to_identifier(segments.last()?);
    if segments.len() < 2 {
        return Some(tail);
    }
    let head = to_identifier(segments.first()?);
    if head == tail {
        return Some(tail);
    }
    Some(format!("{head}_{tail}"))
}

fn strip_extension(segment: &str) -> &str {
    for extension in EXTENSIONS {
        if let Some(stem) = segment.strip_suffix(extension) {
            if !stem.is_empty() {
                return stem;
            }
        }
    }
    segment
}

fn drop_trailing_noise<'s, 'n>(segments: &'s [&'n str]) -> &'s [&'n str] {
    let mut end = segments.len();
    while end > 0 && NOISE.contains(&segments[end - 1]) {
        end -= 1;
    }
    &segments[..end]
}
