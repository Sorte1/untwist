mod common;

use common::PrintedModule;

use oxc_allocator::Allocator;
use untwist_core::pass::Note;
use untwist_unpack::model::Bundle;

fn render(bundle: &Bundle<'_>, notes: &[Note]) -> String {
    let mut out = String::new();
    out.push_str(&format!("type: {}\n", bundle.kind.as_str()));
    out.push_str(&format!(
        "entry: {}\n",
        bundle.entry_id.as_deref().unwrap_or("<none>")
    ));
    if !bundle.chunk_ids.is_empty() {
        out.push_str(&format!("chunks: {}\n", bundle.chunk_ids.join(", ")));
    }
    for note in notes {
        out.push_str(&format!("note: {}\n", note.message));
    }
    for module in bundle.modules.values() {
        out.push_str(&format!(
            "\n=== {} -> {}{}\n",
            module.id,
            module.path.as_str(),
            if module.is_entry { " (entry)" } else { "" }
        ));
        out.push_str(module.code().trim_end());
        out.push('\n');
    }
    out
}

#[test]
fn samples() {
    insta::glob!("../../../fixtures/reference/unpack", "*.js", |path| {
        let source = std::fs::read_to_string(path).expect("read fixture");
        let alloc = Allocator::default();
        let program = common::prepared(&alloc, &source);
        let rendered = match untwist_unpack::unpack(&alloc, &program, &[]) {
            Some(result) => render(&result.bundle, &result.notes),
            None => "no bundle\n".to_string(),
        };
        insta::assert_snapshot!(rendered);
    });
}
