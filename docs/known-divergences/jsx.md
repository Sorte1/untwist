# Known divergences from the reference deobfuscator: JSX

Untwist's JSX reconstruction (`untwist-jsx`) is verified against the reference deobfuscator by `cargo xtask diff-jsx` and in-process by `crates/untwist-jsx/tests/classic.rs`, `tests/automatic.rs`, `tests/pragma.rs` and `tests/fixtures.rs`; the items below are deliberate, not bugs. Both passes are tagged `Safety::Unsafe`, so JSX runs only under `--unsafe` (pinned by `jsx_is_gated_on_the_unsafe_flag`), where the reference deobfuscator converts JSX by default; `diff-jsx` passes `--unsafe` for that reason.

- **The classic runtime's pragma is resolved through its binding by default.** The reference deobfuscator matches the
  literal identifier `React` (`transforms/jsx.ts:32`), so a bundle that imports React under any
  other local name keeps its `createElement` calls (reference deobfuscator issue #10). Untwist's default also
  accepts a binding whose declaration is an `import` of `react`, a `require("react")` (through a
  `.default` interop read as well), or a `window`/`globalThis`/`global`/`self` `.React` property
  read. The conservative comparison mode keeps the reference deobfuscator's literal-name test, so the differential stays an oracle.
  The literal name is also refused by default when it resolves to a variable declarator whose
  initialiser is not React — `var React = { createElement: h }` inside a function — where
  the reference deobfuscator, and the conservative comparison mode, convert the call anyway (`pragma.rs:36-60`, the `shadowing` set
  filled at `pragma.rs:130-139`). Any one-argument wrapper around `require("react")`
  registers the binding — every interop helper (`_interopRequireDefault`, `__importStar`,
  `__importDefault`) takes one argument, so
  `var r = h(require("react"))` binds `r` while `log(1, require("react"), 2)` does not
  (`crates/untwist-jsx/src/pragma.rs:78-92`). Both runtimes resolve the fragment
  through the same bindings by default (`classic.rs`'s `is_fragment_pragma`, `automatic.rs`'s
  `is_fragment`); in the conservative comparison mode the automatic runtime's fragment shortcut matches only the
  literal `React.Fragment`, as the reference deobfuscator's does (`transforms/jsx-new.ts:39`), so
  `import R from "react"; jsxs(R.Fragment, …)` prints `<>…</>` by default and
  `<R.Fragment>…</R.Fragment>` in the conservative comparison mode.
- **A second `Component` uid is spelled `Component_2__`, not `Component2`.** Both tools draw a
  fresh `Component` uid for a lowercase component type they rename and for an element type they
  hoist into a `const` because it has no JSX spelling, and both reserve each uid they hand out, so
  a program that draws the uid twice — two lowercase types, two hoisted types, or one of each —
  gets two distinct names in either tool. The suffix differs: the reference deobfuscator's `generateUid` counts
  (`Component`, `Component2`; `_Component`, `_Component2` on the automatic runtime, where it uses
  Babel's), while Untwist goes through `untwist_core::ast_util::rename::unique_name`, whose
  collision form is `<base>_<n>__` and is shared with every other name Untwist
  invents. No `fixtures/jsx/` fixture draws more than one `Component` uid, so
  `diff-jsx` never sees the difference; the in-process rows
  `two_lowercase_types_get_two_distinct_names` (`tests/classic.rs` and `tests/automatic.rs`) and
  `two_hoisted_types_get_two_distinct_names` (`tests/automatic.rs`) are what pin it, one per path.
- **By default a hoisted element-type declaration is refused when it would land above the scope
  that binds its own free identifiers; the conservative comparison mode keeps the reference deobfuscator's placement.** The automatic
  runtime hoists an element type with no JSX spelling into a `const` declaration keyed to the
  innermost drainable statement, and by default that hoist is refused, with
  `HOIST_ANCHOR_UNRESOLVED`, when a free identifier of the element type resolves to a binding
  declared in a scope that is neither the anchor list's scope nor one of its ancestors: a `let`
  loop head (`for (let i = 0; g(jsx(w[i], {})); i++) h();`), a default parameter
  (`function f(a, b = jsx(pick(a), {}))`) and an arrow parameter (`(x) => jsx(w[x], {})`) are
  left as written, while a `var` loop head (`for (var i = 0; …)`, whose `i` is function-scoped
  and visible at the anchor) and any call whose free identifiers are visible at the anchor still
  hoist. The conservative comparison mode places the declaration exactly where Babel's
  `getStatementParent()` + `insertBefore` places the reference deobfuscator's equivalent: a call inside a statement
  list gets its `const` beside it (`prepare`'s `block-statements` has braced every loop and branch
  body first, so `for (const t of list) f(jsx(t ? "a" : "div", {}))` keeps its declaration inside
  the loop), and the shapes with no statement list between the call and the anchor lift — the
  `let` loop test becomes `const Component = w[i];` above the `for`, where `i` is unbound and
  `w[i]` is read once instead of per iteration; the default parameter hoists
  `const Component = pick(a);` to module level, which throws at load; an arrow expression body is
  evaluated once at definition instead of per call. The hazard is evaluation time as well as
  resolution, and the evaluation-time half — an arrow expression body or a loop test evaluated
  once where the call ran per use — is not a resolution question; it stays inherited from the reference deobfuscator in
  both modes.
- **A fragment child prints inline by default; the conservative comparison mode keeps the reference deobfuscator's container.** The reference deobfuscator's
  `convertChildren` (`transforms/jsx.ts:161-178`) and `convertChild` (`transforms/jsx-new.ts:205-220`)
  pass a child through only when `t.isJSXElement` holds, so a fragment that is a child lands in an
  expression container: `<div>{<>x</>}</div>`. The two forms are the same program, and the inline
  `<div><>x</></div>` is what a hand-written source would say, so Untwist's default prints the
  fragment inline (`build::child_of`, `crates/untwist-jsx/src/build.rs:298`); in the conservative comparison mode the
  fragment child is wrapped as the reference deobfuscator wraps it, so `fixtures/jsx/classic-fragments.js:6` and
  `fixtures/jsx/automatic-members.js:13` hold the conservative half through `diff-jsx` and
  `a_fragment_child_prints_inline_by_default_and_in_a_container_under_conservative` (`tests/classic.rs`
  and `tests/automatic.rs`) pins both modes.
- **A string carrying `&`, `\n` or `\r` goes into an expression container in BOTH modes, where
  the reference deobfuscator prints it as raw JSX text or attribute text.** The reference deobfuscator's `convertAttributes` tests only
  `/["\\]/` before printing a string prop as `title="…"` (`transforms/jsx.ts:143`,
  `transforms/jsx-new.ts:178`) and `convertChildren` tests only `/[{}<>\r\n]/` before printing a
  child as text (`jsx.ts:168`, `jsx-new.ts:211`), so `"a &amp; b"` and `"1 &amp; 2"` print raw as
  `<div title="a &amp; b">1 &amp; 2</div>`, which every JSX compiler decodes back to `a & b` /
  `1 & 2` — a miscompile of the input's meaning — and `"line1\n   line2"` prints as an attribute
  string whose value Babel collapses to one space, spelled with the two characters `\n` from the
  literal's `extra.raw`. Untwist adds `&` to `TEXT_BREAKERS` and `&`, `\r`, `\n` to
  `CONTAINER_CHARS` (`crates/untwist-jsx/src/build.rs:13-14`; `attribute_value`, `build.rs:192`;
  `child_of`, `build.rs:298`), so such a string is emitted as `{"…"}`, which is exact, in both
  modes: the house does not reproduce a miscompile in the conservative comparison mode, so this is a
  recorded divergence rather than a hidden one. No `fixtures/jsx/` fixture carries the shape — it
  would fail the differential by design and the standing rule forbids an allowlist entry — and the
  rows `a_string_with_an_html_entity_is_never_raw_jsx_text_or_attribute_text` and
  `a_string_with_a_newline_is_never_raw_attribute_text` (`tests/classic.rs` and
  `tests/automatic.rs`) pin both modes.
- **JSX is its own pipeline stage rather than a member of the protections group.** The reference deobfuscator merges
  `self-defending`, `debug-protection`, `jsx` and `jsx-new` into one `applyTransforms` call
  (`src/index.ts:201-211`) because the two families share a traversal. Untwist runs them as two
  consecutive fixpoint groups, so `--report json` attributes each pass to the option that enabled
  it and `protections`' per-pass counts keep their shape. The families cannot interact — the
  protections match top-level obfuscator templates and remove statements, JSX matches
  `createElement` and `jsx` calls — so no output moves.
- **A prop shape JSX cannot express refuses that one element and leaves the call.** The reference deobfuscator's
  `convertAttributes` matches an object property by its identifier or string key alone
  (`transforms/jsx.ts:134-137`, `transforms/jsx-new.ts:152-155`), so a computed key is converted
  as if it were static — `React.createElement("div", { [k]: 1 })` prints as `<div k={1} />` —
  a key JSX cannot spell is handed to `t.jsxIdentifier` and printed as invalid JSX
  (`{ "a b": 1 }` becomes `<div a b={1} />`, `createElement("", null)` becomes `< />`), and a
  getter, setter or method reaches the `throw new Error("jsx: property type not implemented …")`
  arm (`transforms/jsx.ts:153-156`, `transforms/jsx-new.ts:166-169`), which aborts the whole
  run. Untwist checks every prop through a borrow before it takes the call apart
  (`build::inconvertible_prop`, `crates/untwist-jsx/src/build.rs:172-183`, over `prop_problem`,
  `build.rs:153-170`) and, on the first method, getter, setter, computed key, lone-surrogate
  string key or key that fails `is_usable_name` (`build.rs:34-40`), leaves the whole element as
  the call it found and records a spanned `ATTRIBUTE_SHAPE` note
  (`crates/untwist-jsx/src/notes.rs:3-5`); a string element type JSX cannot spell as a tag is
  refused the same way under `ELEMENT_NAME_SHAPE` (`notes.rs:6-7`, `build::unspellable_tag`,
  `build.rs:105-116`), and, by default, a string element type whose first character is not an
  ASCII lowercase letter — `createElement("Foo", null)` — is refused under its own
  `COMPONENT_LOOKING_STRING` note, because `<Foo />` recompiles to the component reference
  `createElement(Foo)` (`build::component_looking_string`, `build.rs:118`); the conservative comparison mode converts
  it as the reference deobfuscator's `convertType` does. Both runtimes share the checks
  (`src/classic.rs:75`, `src/automatic.rs:150`). No
  `fixtures/jsx/` fixture carries such a prop, so `diff-jsx` never sees the divergence; the
  in-process rows `a_property_shape_jsx_cannot_express_refuses_the_whole_element`,
  `a_lone_surrogate_prop_key_refuses_the_element_rather_than_dropping_its_props`,
  `a_prop_key_with_a_space_refuses_the_element`, `an_empty_prop_key_refuses_the_element`,
  `a_prop_key_that_looks_like_an_attribute_refuses_the_element` and
  `a_tag_name_jsx_cannot_spell_refuses_the_element` (`crates/untwist-jsx/tests/classic.rs:227-305`;
  the last also in `tests/automatic.rs:295-305`) pin it, and
  `every_refused_shape_is_reported_by_a_catalogued_note` (`tests/classic.rs:308`) holds each
  refusal to its catalogued note.
- **the reference deobfuscator's `props with escaped strings` snapshot pins Babel's `raw` spelling, which Untwist
  cannot promise; the rows themselves are in both harnesses.** The reference deobfuscator's `test/jsx.test.ts:78-91`
  and `test/jsx-new.test.ts:84-97` apply the JSX transform alone (`test/index.ts:9-20`) and
  snapshot Babel's printer, which re-spells each string from `extra.raw` — the source's own
  quotes and escapes (`<Foo bar={'a\\nbc'} />` with the input's single quotes). oxc's printer and
  `ContentEq` derive both print and compare a `StringLiteral` from `value`, never from `raw`, and
  Untwist has no `raw-literals` counterpart, so the snapshot's spelling is the one thing
  about those rows Untwist does not reproduce. That point is confined to the reference deobfuscator's unit-test
  snapshot: through the two CLIs all five rows print identically on both runtimes
  (`<Foo bar={"a\\nbc"} />`, `<Foo bar={"ab\\tc"} />`), because the reference deobfuscator's own `raw-literals`
  pass (`src/unminify/transforms/raw-literals.ts`, applied by `src/unminify/index.ts:7`) has
  cleared `extra.raw` before its printer runs, and `xtask::diff::normalize`
  (`xtask/src/diff.rs:10-14`) re-parses and re-prints both sides through oxc in any case. All
  five rows are therefore in the differential — `fixtures/jsx/classic.js:15-19` and
  `fixtures/jsx/automatic-members.js:8-12` — and in-process by `props_with_escaped_strings`
  (`crates/untwist-jsx/tests/classic.rs:214` and `tests/automatic.rs:259`), where the two
  backslash rows land in an expression container (`build::attribute_value`,
  `crates/untwist-jsx/src/build.rs:192`; `CONTAINER_CHARS`, `build.rs:13`).

