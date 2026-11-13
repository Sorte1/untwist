# Known divergences: Layers

## Inline eval layers

A direct `eval` of a string literal (or of a text the layer folder computes from built-ins) is replaced by the code it runs when the code behaves the same in place. The rule, per site:

- `eval` is the global one: no binding named `eval` anywhere, no write to it, no enclosing `with`, and the program after the rewrite passes the built-in scan for `eval` and the folded built-ins (default lane: intact; `--unsafe`: intact or aliased).
- The text parses as a script under the strictness of the site, has no `return`, `new.target`, `super`, `import()` or `import.meta`, and no label when the site is labelled; `await`/`yield` names are refused inside async functions and generators.
- A statement whose text declares nothing is spliced in place; one that declares only `let`/`const`/`class` is wrapped in a block. A text with `var` or function declarations is spliced only as a direct statement of a function body with simple parameters, when no other code in that function (closures included) names any declared name, the function has no other `eval` or `with`, and the names are not `arguments` or `eval`. In sloppy code the text must not `delete` a name or call `eval` (eval-declared bindings are deletable, inlined ones are not).
- A sloppy site whose text starts with `"use strict"` runs as `(() => { "use strict"; ... })()`, which keeps `this`, `arguments` and the text's own var scope.
- An `eval` used as a value is replaced by `(0, <expression>)` when the text is one expression statement, so a function in it gets no inferred name and a member call loses its receiver as it did.
- `//# sourceURL=` and `//# sourceMappingURL=` comments in the text are dropped.

Divergences: the source text of a function that held the `eval` changes (`Function.prototype.toString`), error stacks no longer name the `sourceURL`, and the inlined code runs one frame shallower.
