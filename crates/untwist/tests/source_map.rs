use untwist::source_map::{self, OwnedSourceMap, Token};
use untwist::{MapLink, Options, Output, Pipeline};

const STRING_ARRAY: &str =
    include_str!("../../../fixtures/reference/deobfuscate/simple-string-array.js");
const CONTROL_FLOW: &str =
    include_str!("../../../fixtures/reference/deobfuscate/obfuscator.io-control-flow.js");
const WEBPACK_4: &str = include_str!("../../../fixtures/reference/unpack/webpack-4.js");
const UNMINIFY: &str = "var a=1,b=2;if(a)b(),a(),c();for(var i=0;i<3;i++)x+=i;";

fn options(source_map: bool) -> Options {
    Options {
        source_map,
        ..Options::default()
    }
}

fn mapped(source: &str) -> Output {
    Pipeline::new(options(true)).run(source)
}

fn utf16_len(line: &str) -> u32 {
    u32::try_from(line.encode_utf16().count()).unwrap_or(u32::MAX)
}

fn reparsed(map: &OwnedSourceMap) -> OwnedSourceMap {
    source_map::parse(&map.to_json_string()).expect("the map parses back")
}

fn assert_in_range(map: &OwnedSourceMap, generated: &str, original: &str) {
    let generated: Vec<&str> = generated.split('\n').collect();
    let original: Vec<&str> = original.split('\n').collect();
    let tokens: Vec<Token> = map.get_tokens().collect();
    assert!(!tokens.is_empty(), "the map holds mappings");
    for token in tokens {
        let line = generated
            .get(token.get_dst_line() as usize)
            .unwrap_or_else(|| panic!("generated line {} is past the end", token.get_dst_line()));
        assert!(
            token.get_dst_col() <= utf16_len(line),
            "generated column in range"
        );
        if token.get_source_id().is_none() {
            continue;
        }
        let line = original
            .get(token.get_src_line() as usize)
            .unwrap_or_else(|| panic!("source line {} is past the end", token.get_src_line()));
        assert!(
            token.get_src_col() <= utf16_len(line),
            "source column in range"
        );
    }
}

fn position_of(text: &str, needle: &str) -> (u32, u32) {
    let at = text
        .find(needle)
        .unwrap_or_else(|| panic!("{needle} is in the output"));
    let before = &text[..at];
    let line = before.matches('\n').count();
    let start = before.rfind('\n').map_or(0, |newline| newline + 1);
    (
        u32::try_from(line).unwrap_or(u32::MAX),
        utf16_len(&before[start..]),
    )
}

fn original_at(map: &OwnedSourceMap, line: u32, column: u32) -> (u32, u32) {
    let table = map.generate_lookup_table();
    let token = map
        .lookup_token(&table, line, column)
        .expect("a mapping covers the position");
    assert_eq!(
        token.get_dst_line(),
        line,
        "the mapping is on the same line"
    );
    (token.get_src_line(), token.get_src_col())
}

fn source_text_at(source: &str, (line, column): (u32, u32)) -> &str {
    let row = source
        .split('\n')
        .nth(line as usize)
        .expect("the source line");
    let prefix: String = row.chars().take(column as usize).collect();
    &row[prefix.len()..]
}

#[test]
fn the_source_map_flag_leaves_the_output_untouched() {
    for source in [STRING_ARRAY, CONTROL_FLOW, WEBPACK_4, UNMINIFY] {
        let plain = Pipeline::new(options(false)).run(source);
        let with_map = mapped(source);
        assert_eq!(plain.codes, with_map.codes);
        assert!(plain.source_maps.iter().all(Option::is_none));
        let modules = |output: &Output| {
            output.bundle.as_ref().map(|bundle| {
                bundle
                    .modules
                    .iter()
                    .map(|module| module.code.clone())
                    .collect::<Vec<_>>()
            })
        };
        assert_eq!(modules(&plain), modules(&with_map));
    }
}

#[test]
fn every_mapping_lies_inside_the_output_and_the_input() {
    for source in [STRING_ARRAY, CONTROL_FLOW, UNMINIFY] {
        let output = mapped(source);
        let map = output.source_maps[0].as_ref().expect("a map");
        let map = reparsed(map);
        assert_in_range(&map, output.code(), source);
        assert_eq!(map.get_source_content(0), Some(source));
    }
}

#[test]
fn a_decoded_string_maps_to_the_array_read_it_replaced() {
    let output = mapped(STRING_ARRAY);
    let map = reparsed(output.source_maps[0].as_ref().expect("a map"));
    let hello = position_of(output.code(), "\"Hello, World!\")");
    let origin = original_at(&map, hello.0, hello.1);
    assert!(
        source_text_at(STRING_ARRAY, origin).starts_with("arr[1]"),
        "{:?}",
        source_text_at(STRING_ARRAY, origin)
    );
    let required = position_of(output.code(), "\"require(\\\"vm\\\")\"");
    let origin = original_at(&map, required.0, required.1);
    assert!(source_text_at(STRING_ARRAY, origin).starts_with("arr3[0] + arr3[3]"));
}

#[test]
fn split_statements_map_to_the_expression_they_came_from() {
    let output = mapped(UNMINIFY);
    let map = reparsed(output.source_maps[0].as_ref().expect("a map"));
    for (printed, original) in [("a();", "a()"), ("c();", "c()"), ("x += i", "x+=i")] {
        let at = position_of(output.code(), printed);
        let origin = original_at(&map, at.0, at.1);
        assert!(
            source_text_at(UNMINIFY, origin).starts_with(original),
            "{printed} maps to {:?}",
            source_text_at(UNMINIFY, origin)
        );
    }
}

#[test]
fn an_inlined_call_maps_to_its_call_site() {
    let source = "function f(p,q){var o={a:function(x,y){return x+y;},c:function(x,y){return x(y);}};return o.c(p,o.a(q,4));}console.log(f(alert,window.n));";
    let output = mapped(source);
    let map = reparsed(output.source_maps[0].as_ref().expect("a map"));
    assert!(
        output.code().contains("return p(q + 4);"),
        "{}",
        output.code()
    );
    for (needle, origin) in [
        ("return p(", "return o.c("),
        ("p(q", "p,o.a(q,4)"),
        ("q + 4", "q,4)"),
    ] {
        let at = position_of(output.code(), needle);
        let found = source_text_at(source, original_at(&map, at.0, at.1));
        assert!(found.starts_with(origin), "{needle} maps to {found:?}");
    }
}

#[test]
fn a_folded_call_maps_to_the_call_it_replaced() {
    let source =
        "function add(x,y){return x+y;}function g(p){return add(p,4);}console.log(g(1),add(7,8));";
    let output = mapped(source);
    let map = reparsed(output.source_maps[0].as_ref().expect("a map"));
    assert!(
        output.code().contains("console.log(g(1), 15);"),
        "{}",
        output.code()
    );
    let at = position_of(output.code(), "15);");
    let found = source_text_at(source, original_at(&map, at.0, at.1));
    assert!(found.starts_with("add(7,8)"), "{found:?}");
}

#[test]
fn every_unpacked_module_gets_a_map_into_the_bundle() {
    let output = mapped(WEBPACK_4);
    let bundle = output.bundle.as_ref().expect("the bundle unpacks");
    assert!(!bundle.modules.is_empty());
    for module in &bundle.modules {
        let map = reparsed(module.map.as_ref().expect("a module map"));
        assert_in_range(&map, &module.code, WEBPACK_4);
        assert_eq!(map.get_source(0), Some(source_map::SOURCE_NAME));
        assert_eq!(map.get_source_content(0), None);
    }
}

#[test]
fn a_module_string_maps_to_the_same_string_in_the_bundle() {
    let output = mapped(WEBPACK_4);
    let bundle = output.bundle.as_ref().expect("the bundle unpacks");
    let module = bundle
        .modules
        .iter()
        .find(|module| module.code.contains("\"#FBC02D\""))
        .expect("a module holds the colour");
    let map = reparsed(module.map.as_ref().expect("a module map"));
    let (line, column) = position_of(&module.code, "\"#FBC02D\"");
    let origin = original_at(&map, line, column);
    assert!(
        source_text_at(WEBPACK_4, origin).starts_with("'#FBC02D'"),
        "{:?}",
        source_text_at(WEBPACK_4, origin)
    );
}

#[test]
fn a_rewritten_require_maps_to_the_module_id_it_replaced() {
    let output = mapped(WEBPACK_4);
    let bundle = output.bundle.as_ref().expect("the bundle unpacks");
    let module = bundle
        .modules
        .iter()
        .find(|module| module.code.contains("require(\"./3.js\")"))
        .expect("a module requires another");
    let map = reparsed(module.map.as_ref().expect("a module map"));
    let (line, column) = position_of(&module.code, "\"./3.js\"");
    let origin = original_at(&map, line, column);
    assert!(
        source_text_at(WEBPACK_4, origin).starts_with("3)"),
        "{:?}",
        source_text_at(WEBPACK_4, origin)
    );
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let value = chunk.iter().enumerate().fold(0u32, |value, (index, byte)| {
            value | (u32::from(*byte) << (16 - 8 * index))
        });
        for index in 0..=chunk.len() {
            out.push(char::from(
                ALPHABET[((value >> (18 - 6 * index)) & 63) as usize],
            ));
        }
        out.push_str(&"=".repeat(3 - chunk.len()));
    }
    out
}

const BUILT: &str = "var a=1;\nconsole.log(a);";
const ORIGINAL_MAP: &str = r#"{"version":3,"sources":["src/app.ts"],"sourcesContent":["let a: number = 1\n\nconsole.log(a)\n"],"names":[],"mappings":"AAAA;AAEA"}"#;

#[test]
fn an_inline_input_map_is_read_back() {
    let source = format!(
        "{BUILT}\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,{}",
        base64(ORIGINAL_MAP.as_bytes())
    );
    assert_eq!(
        Output::input_source_map(&source).as_deref(),
        Some(ORIGINAL_MAP)
    );
    assert_eq!(Output::input_source_map(BUILT), None);
}

#[test]
fn an_input_map_composes_through_to_the_original_sources() {
    let output = Pipeline::new(Options {
        input_source_maps: vec![Some(ORIGINAL_MAP.to_string())],
        ..options(true)
    })
    .run(BUILT);
    let map = reparsed(output.source_maps[0].as_ref().expect("a map"));
    assert_eq!(map.get_source(0), Some("src/app.ts"));
    let at = position_of(output.code(), "console");
    assert_eq!(original_at(&map, at.0, at.1).0, 2);
}

const HOSTILE_MAP: &str = r#"{"version":3,"sources":["input.js","/etc/passwd","../../x.js","file:///etc/hosts","src/app.ts"],"names":[],"mappings":"AAAA;ACEA;ACAA;ACAA;ACAA"}"#;

#[test]
fn source_names_from_an_input_map_stay_data() {
    let built = "var a=1;\nvar b=2;\nvar c=3;\nvar d=4;\nconsole.log(a,b,c,d);";
    let output = Pipeline::new(Options {
        input_source_maps: vec![Some(HOSTILE_MAP.to_string())],
        ..options(true)
    })
    .run(built);
    let map = output.source_maps[0].as_ref().expect("a map");
    let sources: Vec<&str> = map.get_sources().collect();
    assert!(sources.contains(&"src/app.ts"), "{sources:?}");
    for source in &sources {
        assert!(
            *source == "src/app.ts" || source.starts_with(source_map::INPUT_MAP_SCHEME),
            "{source}"
        );
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("input.js");
    let linked = output.inline_linked_codes(std::slice::from_ref(&input));
    let embedded = Output::input_source_map(&linked[0]).expect("an embedded map");
    let written = source_map::parse(&embedded).expect("parses");
    let local = input.to_string_lossy().into_owned();
    assert!(written
        .get_sources()
        .all(|source| source != "input.js" && source != local));
}

#[test]
fn several_inputs_name_their_own_source() {
    let output = Pipeline::new(options(true)).run_many(&["var a=!0;", "var b=!1;"]);
    let names: Vec<_> = output
        .source_maps
        .iter()
        .map(|map| {
            map.as_ref()
                .and_then(|map| map.get_source(0))
                .map(str::to_string)
        })
        .collect();
    assert_eq!(
        names,
        [Some("input1.js".to_string()), Some("input2.js".to_string())]
    );
}

#[test]
fn written_files_carry_a_map_and_a_link() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("bundle.js");
    std::fs::write(&input, WEBPACK_4).expect("write input");
    let out_dir = dir.path().join("out");
    let mut output = mapped(WEBPACK_4);
    output.source_map_link = Some(MapLink::File);
    output
        .write_to_with(&out_dir, std::slice::from_ref(&input), false)
        .expect("write");
    let main = std::fs::read_to_string(out_dir.join("deobfuscated.js")).expect("main file");
    assert!(main.ends_with("//# sourceMappingURL=deobfuscated.js.map"));
    let main_map = std::fs::read_to_string(out_dir.join("deobfuscated.js.map")).expect("map");
    let main_map = source_map::parse(&main_map).expect("parses");
    assert_eq!(main_map.get_source(0), Some("bundle.js"));
    assert_eq!(main_map.get_file(), Some("deobfuscated.js"));
    let bundle = output.bundle.as_ref().expect("bundle");
    for module in &bundle.modules {
        let path = out_dir.join(&module.path);
        let code = std::fs::read_to_string(&path).expect("module file");
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("name");
        assert!(code.ends_with(&format!("//# sourceMappingURL={name}.map")));
        let map = std::fs::read_to_string(format!("{}.map", path.display())).expect("module map");
        let map = source_map::parse(&map).expect("parses");
        assert_eq!(map.get_source(0), Some("bundle.js"));
    }
}

#[test]
fn inline_links_embed_the_map() {
    let output = mapped(UNMINIFY);
    let linked = output.inline_linked_codes(&[]);
    let url = source_map::last_map_url(&linked[0]).expect("a link");
    assert!(url.starts_with("data:application/json"));
    let embedded = Output::input_source_map(&linked[0]).expect("an embedded map");
    let map = source_map::parse(&embedded).expect("parses");
    assert_eq!(map.get_source(0), Some("stdin"));
}

#[test]
fn a_map_stays_on_the_code_after_a_string_with_escaped_hidden_characters() {
    let source = "var a = \"\\u009b\\u009b\\u202e\\t\", b = later(1);\nvar c = `x${\"\\u0085\"}y`, d = after(2);\n";
    let output = mapped(source);
    assert!(
        output.code().contains("\\x9B\\x9B\\u202E\\t"),
        "{}",
        output.code()
    );
    let map = reparsed(output.source_maps[0].as_ref().expect("a map"));
    for name in ["later(1)", "after(2)"] {
        let at = position_of(output.code(), name);
        let origin = original_at(&map, at.0, at.1);
        assert!(
            source_text_at(source, origin).starts_with(name),
            "{name}: {:?}",
            source_text_at(source, origin)
        );
    }
}
