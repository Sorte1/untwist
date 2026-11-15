# Types

## tsc 7.0.2 and well-known-symbol keys

`--types` writes only annotations that hold on every run. The `tools/types-check` gate also asks
that an annotated file get no TypeScript diagnostic its plain text does not get. tsc 7.0.2 does not
always meet that second rule. When unannotated declarators hold type errors that involve a type
printed with a well-known-symbol key (`{ [Symbol.toPrimitive](): number }`, an array of such
objects, a spread copy of one), tsc reports only some of them, and which ones changes from run to
run. An annotation written on one of those declarators makes tsc report its error every time, so the
typed file can show a diagnostic the plain file's run left out.

The annotation is still true and the JavaScript is unchanged; only the checker's report differs.
A file with an object literal keyed by a literal `Symbol.<name>` (`Symbol.iterator`,
`Symbol["toPrimitive"]`) is left untyped (`Refusal::SymbolKey`) to make this rarer. A `Symbol`
reached through another name (`const S = Symbol; ({ [S.iterator]() {} })`), or a class field keyed
by a well-known symbol and copied through a spread or rest, is not caught; no syntactic check
closes every route to such a type.

Reviews: `review/r3-types-r6.md`, `review/r3-types-r7.md`.
