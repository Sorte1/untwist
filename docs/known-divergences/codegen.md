# Known divergences from the reference deobfuscator: Codegen

Untwist prints through `oxc_codegen`, the reference deobfuscator through Babel's generator. The two spell some shapes
differently, and `CodegenOptions` (`oxc_codegen-0.148.0/src/options.rs`) exposes none of them.
Untwist makes a default-on change to the printed text only where every byte differential stays
identical and the shape is measured byte-for-byte against the reference deobfuscator's own CLI output on a synthetic
fixture that then joins the differential.

- **Pretty output matches Babel's array and `function` spelling, not oxc's defaults.** `oxc_codegen`
  breaks an array literal of more than two elements onto one line per element and prints `function(`
  for an anonymous function, `async function(` for an anonymous async one; Babel, which is the reference deobfuscator's
  printer, never breaks an array literal — the longest line in `fixtures/real-world/game.js.snap` is
  170,982 characters — and always prints `function (`. Neither is configurable in `CodegenOptions`,
  so `untwist_core::reflow` re-parses the printed text and edits it by node span **and by the bytes
  at that span** — a method's span starts at its `(`, so an offset alone could land inside a string —
  never by string search, so no byte inside a literal is touched. It runs to a fixpoint so a nested
  literal collapses outer-last. An array with a multi-line element, or a comment between elements,
  keeps its breaks, which is what confines the change to shapes the fixtures lack. `PrintStyle::Compact`
  is untouched, so every `xtask diff-*` harness compares exactly what it compared before.
  `fixtures/unminify/codegen-arrays.js` and `codegen-function-spacing.js` are byte-identical to
  the reference deobfuscator's own output without normalisation. The cost — one re-parse of the printed program per
  fixpoint pass — is small next to the passes themselves.
- **A collapse only crosses gaps that are pure separators.** The text between `[` and the first
  element, between two elements, and between the last element and `]` must be whitespace plus exactly
  the one comma a separator carries (none at the two ends). The printed program is parsed with
  `preserve_parens: false`, so a parenthesised element's span excludes its parentheses — in
  `[(x, y), c, d]` the sequence's span is `x, y` and the "gaps" hold `(` and `)` — and a collapse that
  assumed a gap were only whitespace and a comma would print `[x, y, c, d]`, a four-element array.
  Such an array keeps oxc's one-element-per-line form; Babel prints `[(x, y), c, d]`.
- **An array holding a hole keeps its breaks.** An `Elision`'s span is its own comma, so the gaps
  around it are not separators. Babel spells `[1, , 3, , 5]` as `[1,, 3,, 5]` and `[1, 2, 3, ,]` as
  `[1, 2, 3,,]`; Untwist prints each element on its own line, as oxc does
  (`var a = [\n  1,\n  ,\n  3,\n  ,\n  5\n];`, pinned by `a_sparse_array_keeps_its_lines`). Recorded
  residue.
- **Every comment inside an array literal is dropped.** oxc 0.148's Pretty printer runs with
  `CommentOptions::default()` and prints no comment between `[` and `]`, so `[1, /* two */ 2]`
  comes out as `[1, 2]` where Babel keeps the comment. This is itself a Babel divergence, and it is
  why `holds_a_comment` in `crates/untwist-core/src/reflow.rs` — the guard that refuses to collapse a
  gap holding a comment — is unreachable in practice; it is kept against a later oxc release
  that preserves comments. Recorded residue.
- **An array with a multi-line element keeps every element on its own line.** Babel joins the
  brackets and the one-line elements and lets the last element run on — `[1, 2, function () {\n  x();\n}]`
  — where Untwist prints `[\n  1,\n  2,\n  function () {\n    x();\n  }\n]`. Collapsing it would
  also have to re-indent the element's body, which is the part the reflow deliberately does not do.
  Recorded residue.
- **Numeric literals print in oxc's shortest spelling, not the source's.** `oxc_codegen` never prints
  a `NumericLiteral`'s raw text (`gen.rs:1437-1474`, `print_non_negative_float`), in Pretty as in
  Compact: `0.00002` prints as `2e-5`, `0.000503905` as `503905e-9`, `0.00201562` as `.00201562`.
  Babel prints the raw text. A Compact-normalised differential cannot see this, because both sides
  are re-printed by oxc. Not a reflow concern; recorded as the codegen residue the measurement found.
- **A single-property object literal stays on one line.** oxc prints `{ m: function () {\n  return 6;\n} }`
  where Babel breaks every object with a property: `{\n  m: function () {\n    return 6;\n  }\n}`.
  Two or more properties break on both sides. Not a reflow concern; recorded residue.
- **A surviving self-defending check's read target is printed on one line.** obfuscator.io's `selfDefending` emits three checks that read a function's
  own source. Pretty printing that function across lines makes each check fail, so the printed
  program hangs or throws where the input ran. During the first reflow pass, a variable declarator
  (`var`, `let` or `const`) whose initialiser is one of these templates, or an expression statement
  `<name> = <template>` (a plain `=` to a bare name; not `+=`, `??=`, `||=` or `&&=`, a
  destructuring or member target, a chained `a = b = <template>`, a class field, or an assignment
  inside a larger expression), has the function its check reads printed on one line:
  - **G1, the single-call controller** (`var f = <true>; return function (c, fn) { var r = f ?
    function () { … if (fn) { var x = fn[<any key>](c, arguments); fn = null; return x; } } :
    function () {}; f = <false>; return r; }`, with the sequence forms `return f = …, r` and
    `return fn = null, x`). Its target is `r`. Its reader searches `(((.+)+)+)+$`, which matches any
    text without a line terminator and backtracks exponentially on one that has a break.
  - **G2, the `newState` constructor** (`this.<k> = p; this.<k> = [1, 0, 0]; this.<k> = function ()
    { return "newState"; }; this.<k> = "\w+ *\(\) *{\w+ *"; this.<k> = "['|\"].+['|\"];? *}"`, as
    five statements or one sequence). Its target is the `newState` function. The regex built from
    the two strings matches `function () {return "newState";}` exactly as it matches the obfuscator's
    `function(){return'newState';}`.
  - **G3, the self-reading base64 helper** (an optional leading 65-character alphabet, which
    terser inlines into the `indexOf` call, then `var a = "", b = "", c = a + <id>` as one or
    three declarations, and a `for` holding `c.charCodeAt(<e> + 10) - 10 !== 0`, or terser's
    `!=`). Its target is the helper itself. That test asks only whether the character at each
    offset is a line feed, so it is constant-true on any single-line text.

  Every template is also matched at its own function wherever that function stands (a `return`
  argument, a call argument, a branch, uglify's `x.k = e = function (i) { … }` inside a sequence),
  not only in the declarator and assignment slots above. Before this (R51-SD) a G2 or G3 template
  that uglify or terser had folded into a sequence was left across lines, and 90 of the 240
  default, `--unsafe` and reprint runs of the obfuscator-versions matrix's selfDefending cells
  printed something else or hung. For G1, obfuscator.io's dead-code injection wraps the
  controller in `if (<opaque test>) { … } else { <junk> }` (terser: `if (!t) { … } { <junk> }`),
  around the `var f = true; return function (c, fn) { … }` pair and around the choice inside the
  two-parameter function, and sometimes puts a decoy copy of the template (with foreign names)
  in the other branch; control-flow flattening, string-array wrappers and transformObjectKeys put
  proxy tables (`var t = { k: function (a, b) { var w = d; return a === b; } }`), aliases and
  split tables (`var h = {}; h[k] = v;`) in front of the choice and of the forwarding call. So
  the two-parameter function may hold any statements before the choice, the choice may sit up
  to four `if` branches deep in either branch (every branch is tried, and
  the decoy does not match), and the target `r` only has to hold the forwarding call
  `fn[<any key>](c, arguments)` and the reset `fn = null` somewhere outside its nested
  functions. The flag, the empty alternate `function () {}` and the closing `f = false; return
  r` (or `return f = false, r`, hoisted function declarations skipped, unreachable code after it
  allowed) stay exact. A function of this shape that is not a guard is
  printed on one line too; that costs readability, never behaviour, since a one-line print is
  used only when it re-parses to the same tree (`reflow_guard_shapes.rs`,
  `fixtures/self-defending`).

  The templates also accept leading constant tables (`var k = { a: "0x37a" }`) before the controller
  and inside `r`, and numbers written as literal arithmetic (`-1385 * -1 + -7397 + 6013`), which is
  how obfuscator.io's numbers-to-expressions prints them. Every other anchor stays exact, and a
  near-miss keeps today's text.

  The one-line text is the function printed Pretty with no comments, indent 0 and its line breaks
  removed, then reflowed to a fixed point, so a later pass never changes it. It is used only if
  re-parsing it gives oxc's minified print of the original function (AST equality). Otherwise, as for
  a target holding a raw line break in a template literal, the function keeps today's lines and the
  note "a self-defending check that reads a function's own source survived, but that function holds
  a line break no one-line print can remove" is emitted. Applies in every Pretty mode, the conservative comparison mode
  included; the Compact differentials are untouched.

  Each guard is noted once per input. A bundle module is printed again from its tree, so its reflow
  meets a guard the input's reflow already printed on one line (or kept) across lines again; the
  module's text is rewritten the same way, but the guard is not counted a second time (5b row 26).
  The count keys a guard on its one-line text with the names it binds or reads replaced by their
  order of first use, so a guard whose names unpack renames (for example a check that reads a
  renamed module parameter) is not counted again (`bundle.rs`,
  `a_guard_whose_text_unpack_renames_is_noted_once`). Numbers, strings, regular expressions (by
  the previous-token rule), property names after `.` and object or method keys are kept as written,
  and a template's substitutions are read as code, so two guards that differ in any of those are
  counted apart (`reflow_guards.rs`). A guard that unpack changes in any other way, or whose
  regular expression spells a renamed name, is counted again.

  For comparison, Babel's printer breaks the same guards in the reference deobfuscator's `deobfuscate: false` output.
  When a unit decodes, protections removes the guards as the reference deobfuscator does, so nothing is left to print.

  **Premise.** A surviving template is printed as the obfuscator emits it, on one line. For an
  input whose guard already fires when Untwist reads it, Untwist's output runs where the input hangs
  or throws. That input is either multi-line text (beautified obfuscator.io output), or, for G2 only,
  any single-line spelling of the `newState` function that its regex rejects, such as a space after
  `{` (`function(){ return 'newState'; }`): the compact print passes the check where the input
  failed it. G1 and G3 do not depend on spacing, and the direction is always fail to pass
  (`guard_behaviour.rs`, `a_single_line_state_function_its_regex_rejects_is_the_documented_premise`).
  Proving the input text single-line would need a walk on every row and the arena kept alive across
  the reflow; obfuscator.io never emits the template multi-line or spaced. A span-based check that
  rides an existing walk could lift this premise.
- **A G1 check whose read target would still print across lines passes the input through.**
  The G1 templates above are a recogniser, so a near-miss used to leave the target across lines
  and the output hung (javascript-obfuscator 5.10.1 with dead-code injection, R51-SD). Every input
  is now also checked for the G1 reader: a `var n = c(this, <parameterless function or arrow>)`
  (block or expression body, extra arguments after the callback allowed),
  `n = c(this, …)` or the control-flow proxy form `n = t.k(c, this, …)`, whose callback holds a
  chain of at least three calls that passes `n` back in
  (`n.toString().search(r).toString().constructor(n)…`, or the older
  `Q.constructor("return /" + this + "/")().constructor(r).test(n)`; string-array calls in the
  keys do not matter). The check runs on the final printed text, not on what the reflow meant to
  print: when the final tree holds a reader, the output is parsed again with scope analysis. `c`
  is resolved by binding, not by name. It must be a `var`/`let`/`const` binding (no parameter,
  function, class, catch or import binding of it, no destructuring) with exactly one value, its
  initialiser or one plain `c = …` assignment, and no other write; a program with a direct `eval`
  or a `with` statement has no resolved controller. When `c` is not a name, the callee itself is
  the controller. Every two-parameter controller target inside that value, in every branch the
  template matches, must be printed without a line break in the output. If the binding does not
  qualify, no target is found, or one is printed across lines, the input is passed through
  unchanged with the note "a self-defending check reads the source text of a function the output
  prints across lines (or one it cannot find), so the reformatted output would hang where the
  input runs; it is passed through unchanged". The tree walk that finds readers is unchanged; the
  second parse runs only for programs that hold one. A controller written as an arrow (terser's
  `unsafe_arrows`) is not a template, so those programs are passed through.
  Older obfuscator.io lines (0.x) also test, inside the same `c(this, cb)` callback, that a bait
  `function () { return "\x77\x69…"; }` keeps its hex escapes; no reprint keeps a string's
  escapes, so a callback holding a parameterless function returning `"window"` (assigned or called
  in place, as terser leaves it) passes the input through. The bait functions themselves
  (a parameterless function whose whole body is `return "dev"`, `"window"` or `"newState"`, the
  words obfuscator.io's checks read) are printed on one line wherever they stand.
  A function that adds its own name to a string (`c = a + F` inside `F`, declared or assigned,
  the G3 self-read) is a reader too: it must come out on one line the same way. Its
  `("" + function () { … }).indexOf("\n")` escape hatch only counts when the escape function is
  printed with a line break in the output; an escape printed on one line (an empty function, an
  expression arrow, a bait) leaves the self-read live. Every unpacked module is checked the same
  way on its own printed text; a module that fails makes the inputs be written without a bundle.
  When a stage panics, the tree printed instead is checked too. A source check of another shape
  (another obfuscator's, or a read through a member such as `F.toString()` outside G1) is not
  seen; see the next item.
- **Function source text is not kept (the printer premise).** Untwist's output is a new spelling
  of the program in every mode, `--no-deobfuscate --no-unminify` included, so
  `Function.prototype.toString`, `String(f)`, `"" + f` and template substitutions of a function
  see the printed text, not the input's. A program whose behaviour depends on that text (a hash
  or length of a function, a comparison, a regular expression run over it) can behave
  differently after any print. The safe lane's behaviour-equality promise holds modulo function
  source text. The exceptions are the self-reading checks obfuscator.io emits: the G1-G3 read
  targets and the cookie check are printed on one line as the obfuscator emitted them, and a G1
  reader whose target cannot be printed that way passes the input through (item above). The
  protections stage removes these checks where it proves the callback is obfuscator.io's own; the
  one-line print and the pass-through are for the lanes and programs where it does not.
