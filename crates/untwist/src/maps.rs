use std::path::{Path, PathBuf};

use untwist_core::source_map::{self, OwnedSourceMap};

use crate::pipeline::Output;

pub const MAP_SUFFIX: &str = ".map";
const URL_COMMENT: &str = "//# sourceMappingURL=";
const STDIN_NAME: &str = "stdin";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapLink {
    File,
    Inline,
}

pub struct Linked {
    pub code: String,
    pub map: Option<(String, String)>,
}

fn input_file_name(input: &Path) -> String {
    input
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| STDIN_NAME.to_string())
}

fn source_for(name: &str, total: usize, inputs: &[PathBuf]) -> String {
    let Some(index) = source_map::input_index(name, total) else {
        return name.to_string();
    };
    match inputs.get(index) {
        Some(input) => input_file_name(input),
        None if total <= 1 => STDIN_NAME.to_string(),
        None => name.to_string(),
    }
}

fn resolved(map: &OwnedSourceMap, file: &str, total: usize, inputs: &[PathBuf]) -> OwnedSourceMap {
    let mut map = map.clone();
    let sources: Vec<String> = map
        .get_sources()
        .map(|name| source_for(name, total, inputs))
        .collect();
    map.set_sources(sources);
    map.set_file(file);
    map
}

fn is_url_safe(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'~' | b'-')
}

fn percent_encoded(name: &str) -> String {
    let mut encoded = String::with_capacity(name.len());
    for byte in name.bytes() {
        match is_url_safe(byte) {
            true => encoded.push(char::from(byte)),
            false => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn file_name(relative: &str) -> &str {
    relative.rsplit('/').next().unwrap_or(relative)
}

pub struct Target<'t> {
    pub relative: &'t str,
    pub inputs: &'t [PathBuf],
    pub total: usize,
}

pub fn link(code: &str, map: &OwnedSourceMap, link: MapLink, target: &Target<'_>) -> Linked {
    let name = file_name(target.relative);
    let map = resolved(map, name, target.total, target.inputs);
    let separator = if code.is_empty() || code.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    match link {
        MapLink::Inline => Linked {
            code: format!("{code}{separator}{URL_COMMENT}{}", map.to_data_url()),
            map: None,
        },
        MapLink::File => {
            let map_name = percent_encoded(&format!("{name}{MAP_SUFFIX}"));
            Linked {
                code: format!("{code}{separator}{URL_COMMENT}{map_name}"),
                map: Some((
                    format!("{}{MAP_SUFFIX}", target.relative),
                    map.to_json_string(),
                )),
            }
        }
    }
}

impl Output {
    pub fn input_source_map(source: &str) -> Option<String> {
        source_map::inline_map(source)
    }

    pub fn source_map_json(&self, index: usize) -> Option<String> {
        self.source_maps
            .get(index)?
            .as_ref()
            .map(OwnedSourceMap::to_json_string)
    }

    pub fn inline_linked_codes(&self, inputs: &[PathBuf]) -> Vec<String> {
        let total = self.codes.len();
        self.codes
            .iter()
            .enumerate()
            .map(|(index, code)| match self.source_maps.get(index) {
                Some(Some(map)) => {
                    let target = Target {
                        relative: "stdout.js",
                        inputs,
                        total,
                    };
                    link(code, map, MapLink::Inline, &target).code
                }
                _ => code.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{link, MapLink, Target, URL_COMMENT};
    use std::path::PathBuf;
    use untwist_core::source_map;

    fn linked_url(relative: &str) -> (String, String) {
        let map = source_map::identity("a();");
        let target = Target {
            relative,
            inputs: &[],
            total: 1,
        };
        let linked = link("a();", &map, MapLink::File, &target);
        let (_, url) = linked.code.rsplit_once(URL_COMMENT).expect("a link");
        (linked.code.clone(), url.to_string())
    }

    #[test]
    fn a_line_break_in_a_module_name_stays_inside_the_link() {
        for name in [
            "a\nevil();x.js",
            "a\r\nb.js",
            "a\u{2028}b.js",
            "a\u{2029}b.js",
        ] {
            let (code, url) = linked_url(name);
            assert_eq!(code.lines().count(), 2, "{code:?}");
            assert!(
                !url.contains(['\n', '\r', '\u{2028}', '\u{2029}']),
                "{url:?}"
            );
        }
    }

    #[test]
    fn the_link_is_percent_encoded() {
        assert_eq!(linked_url("dir/a b#1 50%.js").1, "a%20b%231%2050%25.js.map");
        assert_eq!(linked_url("plain-name_1.js").1, "plain-name_1.js.map");
        assert_eq!(linked_url("a\nevil();x.js").1, "a%0Aevil%28%29%3Bx.js.map");
    }

    #[test]
    fn the_source_is_the_input_file_name_without_its_directory() {
        let map = source_map::identity("a();");
        let inputs = [PathBuf::from("/home/alice/clients/acme/in.js")];
        let target = Target {
            relative: "deobfuscated.js",
            inputs: &inputs,
            total: 1,
        };
        let linked = link("a();", &map, MapLink::File, &target);
        let (_, json) = linked.map.expect("a map file");
        let written = source_map::parse(&json).expect("parses");
        assert_eq!(written.get_source(0), Some("in.js"));
    }
}
