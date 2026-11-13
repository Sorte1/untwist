#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PassInfo {
    pub name: &'static str,
    pub stage: &'static str,
    pub unsafe_only: bool,
}

const fn pass(name: &'static str, stage: &'static str, unsafe_only: bool) -> PassInfo {
    PassInfo {
        name,
        stage,
        unsafe_only,
    }
}

pub const PASSES: &[PassInfo] = &[
    pass("block-statements", "prepare", false),
    pass("sequence", "prepare", false),
    pass("split-variable-declarations", "prepare", false),
    pass("find-string-array", "deobfuscate", false),
    pass("find-decoders", "deobfuscate", false),
    pass("inline-object-props", "deobfuscate", false),
    pass("inline-decoder-wrappers", "deobfuscate", false),
    pass("inline-decoded-strings", "deobfuscate", false),
    pass("remove-decoder-machinery", "deobfuscate", false),
    pass("unstamp-remaining-machinery", "deobfuscate", false),
    pass("layers", "deobfuscate", false),
    pass("inline-layers", "deobfuscate", false),
    pass("default-parameters", "transpile", false),
    pass("logical-assignments", "transpile", false),
    pass("nullish-coalescing", "transpile", false),
    pass("nullish-coalescing-assignment", "transpile", false),
    pass("optional-chaining", "transpile", false),
    pass("template-literals", "transpile", true),
    pass("computed-properties", "unminify", false),
    pass("for-to-while", "unminify", false),
    pass("infinity", "unminify", false),
    pass("invert-boolean-logic", "unminify", false),
    pass("json-parse", "unminify", false),
    pass("logical-to-if", "unminify", false),
    pass("merge-else-if", "unminify", false),
    pass("merge-strings", "unminify", false),
    pass("number-expressions", "unminify", false),
    pass("remove-double-not", "unminify", false),
    pass("split-for-loop-vars", "unminify", false),
    pass("string-literal-in-template-literal", "unminify", false),
    pass("ternary-to-if", "unminify", false),
    pass("truncate-number-literal", "unminify", false),
    pass("typeof-undefined", "unminify", false),
    pass("unary-expressions", "unminify", false),
    pass("unminify-booleans", "unminify", false),
    pass("void-to-undefined", "unminify", false),
    pass("yoda", "unminify", false),
    pass("unwrap-comma-call", "unminify", false),
    pass("untranspile-helpers", "untranspile", false),
    pass("self-defending", "protections", false),
    pass("debug-protection", "protections", false),
    pass("jsx", "jsx", true),
    pass("jsx-new", "jsx", true),
    pass("dead-code", "cleanup", false),
    pass("control-flow-object", "cleanup", false),
    pass("control-flow-switch", "cleanup", false),
    pass("merge-object-assignments", "cleanup", false),
    pass("evaluate-globals", "cleanup", false),
    pass("fold-pure-calls", "cleanup", false),
    pass("var-transformation", "cleanup", false),
    pass("dead-assignment-elimination", "cleanup", false),
    pass("fold-literal-chains", "cleanup", false),
    pass("let-to-const", "lexical", false),
    pass("var-to-let", "lexical", false),
    pass("function-to-arrow", "lexical", false),
    pass("merge-member-declarations", "lexical", false),
    pass("name-commonjs-wrappers", "rename", false),
    pass("rename-simple", "rename", false),
    pass("rename-smart", "rename", false),
    pass("rename-unique", "rename", false),
    pass("mangle", "rename", false),
    pass("rename-shorthand", "rename", false),
];

pub fn pass_named(name: &str) -> Option<&'static str> {
    PASSES
        .iter()
        .find(|pass| pass.name == name)
        .map(|pass| pass.name)
}
