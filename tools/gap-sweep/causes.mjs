export const CAUSES = [
  ["direct-eval", "a direct eval or `with` can see the unit", /direct eval|with statement/, "decoder"],
  ["debug-protection", "debug protection kept: a reference survives outside the removed statements", /debug protection function was found/, "decoder"],
  ["var-bound", "the string array or a decoder is a `var` function expression, not a declaration", /not a function declaration, so its binding may still be unset/, "decoder"],
  ["redeclared", "the array or decoder name is redeclared, block-declared or reassigned", /declared again|declared in a block|assigned outside its own body/, "decoder"],
  ["builtin-replaced", "the program replaces a built-in the decoder or rotator uses", /replaces a standard built-in/, "decoder"],
  ["global-reach", "code holding the global object, or code from text, may reach the machinery", /global object, or code from text|names of the global object/, "decoder"],
  ["spelled-name", "a string in the program spells the machinery's name", /spells this function's name|decoded string spells/, "decoder"],
  ["code-before-rotator", "code that may run precedes the rotator", /precedes the rotator/, "decoder"],
  ["rotator-effects", "the rotator touches bindings outside the unit", /rotator reads or writes something other/, "decoder"],
  ["rotator-placement", "the rotator is not a statement of the program body", /rotator is not a statement of the program body/, "decoder"],
  ["array-escapes", "the string array is reachable from code outside the unit", /reachable from code outside its declaration/, "decoder"],
  ["decoder-impure", "a decoder escapes, mutates the array or reads outer state", /a decoder of the unit (is used other|reads a binding|does more than)/, "decoder"],
  ["stateful-decoder", "a stateful decoder whose state is not a proven memo or init gate", /stateful decoder/, "decoder"],
  ["evaluator-gap", "the evaluator bails on a construct it does not model", /decoding bailed|unsupported construct|did not decode to a string/, "decoder"],
  ["simple-array-access", "a simple string array is read other than by literal index, so it is not inlined", /simple string array not inlined/, "index"],
  ["object-props-refused", "an object of proxy functions or constants was left in place", /inline-object-props:/, "proxy"],
  ["unsafe-only", "the residue clears only under --unsafe (dead-code injection hides it from the default lane)", /$^/, "decoder"],
];

const FAMILIES = {
  decoder: ["stringArrays", "rotators", "decoderCalls"],
  index: ["arrayIndex"],
  cff: ["cffSwitches", "opaquePredicates"],
  proxy: ["proxyObjects"],
  dynamic: ["evalCalls", "functionCtors"],
  encoder: ["packer", "jsfuck", "jjencode", "aaencode"],
};

export function dominantFamily(residue) {
  const scored = Object.entries(FAMILIES).map(([family, keys]) => [family, keys.reduce((total, key) => total + (residue?.[key] ?? 0), 0)]);
  const [family, score] = scored.sort((a, b) => b[1] - a[1])[0];
  return score > 0 ? family : undefined;
}

export function causeOf(notes, residue) {
  const text = notes.join("\n");
  const family = residue ? dominantFamily(residue) : undefined;
  const candidates = family ? CAUSES.filter((cause) => cause[3] === family) : CAUSES;
  const hit = candidates.find(([, , pattern]) => pattern.test(text));
  if (hit) return hit[0];
  return family ? `no-note:${family}` : "unexplained";
}

export function familyCounts(residue) {
  return Object.fromEntries(Object.entries(FAMILIES).map(([family, keys]) => [family, keys.reduce((total, key) => total + (residue?.[key] ?? 0), 0)]));
}

export function causeInFamily(notes, family) {
  const text = notes.join("\n");
  const hit = CAUSES.find(([, , pattern, owner]) => owner === family && pattern.test(text));
  return hit ? hit[0] : `no-note:${family}`;
}

const OUTCOMES = {
  "no-string-array": "untwist exits 1: no string array found, nothing to decode (HTML error pages and plain loaders)",
  timeout: "untwist timed out",
};

export function causeLabel(key) {
  if (OUTCOMES[key]) return OUTCOMES[key];
  if (/^exit-/.test(key)) return `untwist exited with status ${key.slice(5)}`;
  const named = CAUSES.find(([name]) => name === key);
  if (named) return named[1];
  const family = key.replace(/^no-note:/, "");
  return `no refusal note; ${FAMILIES[family]?.join("/") ?? family} markers left`;
}
