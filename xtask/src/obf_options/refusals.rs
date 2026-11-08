pub const INTERPRETED: &str = "no checksum could be extracted from the rotator";

pub const REFUSALS: [(&str, &str, &str); 13] = [
    (
        "reads a binding outside itself",
        "decoder-reads-outer-binding",
        "a decoder reads a binding outside itself that is not a standard global (the old atob polyfill's window fallback), so the unit's calls are left alone",
    ),
    (
        "a decoder of the unit is used other than as the callee of a call",
        "decoder-used-as-value",
        "a decoder of the unit is used other than as the callee of a call, so it may run from where no call site was collected, and the unit's calls are left alone",
    ),
    (
        "the string array is reachable from code outside its declaration",
        "array-reached-outside",
        "the string array is reachable from code outside its declaration, decoders and a recognised rotator, so the unit's calls are left alone",
    ),
    (
        "a stateful decoder keeps state that is not a proven memo or init gate",
        "stateful-decoder-state",
        "decoding bails: a stateful decoder keeps state that is not a proven memo or init gate",
    ),
    (
        "a decoder of the unit does more than read from the string array",
        "decoder-does-more",
        "a decoder does more than read from the string array, so its calls may change or expose the array and are left alone",
    ),
    (
        "the rotator reads or writes something other than its own bindings",
        "rotator-reads-outside",
        "the rotator reads or writes something other than its own bindings, the string array and its decoders, so its effect is not proven",
    ),
    (
        "the alias is not declared exactly once",
        "wrapper-alias-redeclared",
        "a decoder alias is not declared exactly once by a single-declarator declaration in a statement list, so it is left in place",
    ),
    (
        "the alias may be read before its declaration runs",
        "wrapper-alias-read-early",
        "a decoder alias may be read before its declaration runs, so it is left in place",
    ),
    (
        "a decoded string spells the name of this string array",
        "decoded-spells-machinery",
        "a decoded string spells the name of the string array or a decoder, so code built from it may reach the machinery",
    ),
    (
        "the wrapper body reads something other than its own parameters",
        "wrapper-reads-outside",
        "a decoder wrapper body reads something other than its own parameters and the stamped machinery, so it is not inlined",
    ),
    (
        "the wrapper's binding is written again, or it is declared in a block",
        "wrapper-binding-unstable",
        "a decoder wrapper is declared in a block (uglify-js inlines the rotator into one) or its binding is written again, so a call may reach another function and the wrapper is left in place",
    ),
    (
        "a lexical binding that may be read before its declaration runs",
        "decoder-reads-tdz-binding",
        "decoding bails: a decoder wrapper reads a const of the rotator that the evaluator cannot prove is initialised when the wrapper runs",
    ),
    (
        "this plain string array may be reached by name through the global object",
        "legacy-array-reached",
        "safe lane: code may reach the plain string array by name through the global object (an undecoded obj[dec(k)](...) may run code), a with statement or a direct eval, so it is not lifted and the unit is left undecoded",
    ),
];

pub const RESIDUES: [(&[&str], &str, &str); 3] = [
    (
        &["opaque_predicates"],
        "missed-opaque-predicate",
        "MISSED: the unit decodes, but a dead-code-injection test between two different string literals is left and no pass says why",
    ),
    (
        &["call_controllers", "self_defending"],
        "missed-self-defending",
        "MISSED: the unit decodes, but self-defending code is left and no pass says why",
    ),
    (
        &["proxy_objects"],
        "missed-proxy-object",
        "MISSED: the unit decodes, but a control-flow proxy object is left and no pass says why",
    ),
];

pub fn refusal_of(note: &str) -> Option<&'static str> {
    REFUSALS
        .iter()
        .find(|(needle, _, _)| note.contains(needle))
        .map(|(_, key, _)| *key)
}

fn classes(detail: &str) -> Option<Vec<&str>> {
    detail
        .split(',')
        .map(|part| {
            let (class, count) = part.split_once('=')?;
            (!count.is_empty() && count.bytes().all(|b| b.is_ascii_digit())).then_some(class)
        })
        .collect()
}

pub fn residue_of(detail: &str) -> Option<&'static str> {
    let found = classes(detail)?;
    RESIDUES
        .iter()
        .find(|(allowed, _, _)| found.iter().all(|class| allowed.contains(class)))
        .map(|(_, key, _)| *key)
}

pub fn reasons() -> impl Iterator<Item = (&'static str, &'static str)> {
    REFUSALS
        .iter()
        .map(|(_, key, reason)| (*key, *reason))
        .chain(RESIDUES.iter().map(|(_, key, reason)| (*key, *reason)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_note_and_a_silent_residue_name_their_causes() {
        assert_eq!(
            refusal_of("note: inline-decoder-wrappers: the alias may be read before its declaration runs, so it was left in place"),
            Some("wrapper-alias-read-early")
        );
        assert_eq!(refusal_of("note: something else"), None);
        assert_eq!(
            residue_of("self_defending=2,call_controllers=2"),
            Some("missed-self-defending")
        );
        assert_eq!(
            residue_of("opaque_predicates=1"),
            Some("missed-opaque-predicate")
        );
        assert_eq!(residue_of("proxy_objects=1,decoders=1"), None);
        assert_eq!(residue_of("-"), None);
    }
}
