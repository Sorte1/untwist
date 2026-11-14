# Known divergences: Layers

## Inline eval layers

A direct `eval` of a string literal (or of a text the layer folder computes from built-ins) is replaced by the code it runs when the code behaves the same in place. The rule, per site:

- `eval` is the global one: no binding named `eval` anywhere, no write to it, no enclosing `with`, no other sloppy direct `eval` whose text may declare `eval` in the site's function or one around it (that includes a peeled text that itself calls `eval`), and the program after the rewrite passes the built-in scan for `eval` and the folded built-ins (default lane: intact; `--unsafe`: intact or aliased).
- The text parses as a script under the strictness of the site and passes its early-error checks, every regular expression literal in it is valid on every engine (only the flags `gimsuy`; no lookbehind, property escapes, `\k`, modifiers or repeated group names; a pattern the evaluator's own grammar or its engine accepts), has no `return`, `new.target`, `super`, `import()` or `import.meta`, and no label when the site is labelled; `await`/`yield` names are refused inside async functions and generators.
- A statement whose text declares nothing is spliced in place; one that declares only `let`/`const`/`class` is wrapped in a block. A text with `var` or function declarations is spliced only as a direct statement of a function body with simple parameters, when no other code in that function (closures included) names any declared name, the function has no other `eval` or `with`, and the names are not `arguments` or `eval`. In sloppy code the text must not `delete` a name or call `eval` (eval-declared bindings are deletable, inlined ones are not).
- A sloppy site whose text starts with `"use strict"` runs as `(() => { "use strict"; ... })()`, which keeps `this`, `arguments` and the text's own var scope. Where the site is the body of an `if`, a loop or a label, the call is wrapped in a block.
- A site inside a class field initialiser or static block, or inside an arrow function in one, is kept (there `arguments` in the text is an early error).
- An `eval` used as a value is replaced by `(0, <expression>)` when the text is one expression statement, so a function in it gets no inferred name and a member call loses its receiver as it did.
- `//# sourceURL=` and `//# sourceMappingURL=` comments in the text are dropped.

Divergences: the source text of a function that held the `eval` changes (`Function.prototype.toString`), error stacks no longer name the `sourceURL`, and the inlined code runs one frame shallower.

Premise: the text is judged by the grammar of the engine that runs the output. Syntax only some engines accept (optional chaining, private names, newer regular expression features) is inlined when it parses, so a feature test such as `try { eval("a?.b") } catch {}` loses its fallback branch on an engine without it. Regular expressions are the exception above.

## Function layers

`Function(params, body)(args)` is peeled to `(function anonymous(params) { body })(args)`. That function expression binds `anonymous` inside itself, which the function `Function` makes does not, so the layer is kept when the body names `anonymous` or contains a direct `eval` (which could read the name at run time).

Built-in scan: a prototype counts as reached when it is read under `__proto__`, `constructor`, `__lookupGetter__` or `__lookupSetter__`, also through a destructuring pattern (declaration, parameter or assignment) and under any computed key that is not provably something else. In the default lane that means a number, a string literal other than those four names, a well-known symbol, a literal concatenation, `atob`/`String.fromCharCode` of literals, or a variable whose every write is one of these; any other key (a parameter, a member read, an enumeration, a global, a call) counts as possibly reaching, so a program that reads through one and lets the value escape is not trusted. `--unsafe` premise: such an opaque key spells none of the four names unless the program holds one of them as a whole string literal or template; keys built from strings (a concatenation with no number operand, a template with substitutions, a string-method or `String.fromCharCode`-style call, or a variable written from such values or from the name itself) count in both lanes.
