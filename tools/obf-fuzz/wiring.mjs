export const ALIAS_KINDS = ["var", "let", "const", "object", "wrapper", "iife", "function-expression", "comma", "chain"];
const INERT_GLOBAL = ["var", "function"];
const INERT_LOCAL = ["void", "empty", "block"];

function walk(node, visit, parent = null, key = null) {
  if (!node || typeof node.type !== "string") return;
  visit(node, parent, key);
  for (const [field, value] of Object.entries(node)) {
    if (Array.isArray(value)) {
      for (const child of value) walk(child, visit, node, field);
    } else if (value && typeof value.type === "string") {
      walk(value, visit, node, field);
    }
  }
}

function isBindingSite(node, parent, key) {
  if (!parent) return false;
  if (parent.type === "LabeledStatement" || parent.type === "BreakStatement" || parent.type === "ContinueStatement") return true;
  if (parent.type === "ExportSpecifier" || parent.type === "ImportSpecifier" || parent.type === "ImportDefaultSpecifier" || parent.type === "ImportNamespaceSpecifier") return true;
  if (parent.type === "MemberExpression" && key === "property" && !parent.computed) return true;
  if ((parent.type === "Property" || parent.type === "MethodDefinition" || parent.type === "PropertyDefinition") && key === "key" && !parent.computed) return true;
  return false;
}

function stringCount(node) {
  let count = 0;
  walk(node, (n) => {
    if (n.type === "Literal" && typeof n.value === "string") count++;
  });
  return count;
}

function references(node, name) {
  let found = false;
  walk(node, (n, parent, key) => {
    if (n.type === "Identifier" && n.name === name && !isBindingSite(n, parent, key)) found = true;
  });
  return found;
}

export function machinery(ast) {
  const functions = ast.body.filter((s) => s.type === "FunctionDeclaration" && s.id);
  const arrays = functions.filter((f) => {
    let has = false;
    walk(f.body, (n) => {
      if (n.type === "ArrayExpression" && n.elements.length >= 2 && stringCount(n) >= 2) has = true;
    });
    return has;
  });
  const arrayNames = arrays.map((f) => f.id.name);
  const decoders = functions.filter((f) => !arrayNames.includes(f.id.name) && arrayNames.some((a) => references(f.body, a)));
  return { arrays, decoders };
}

function freshName(r, used) {
  for (;;) {
    const name = `_0x${r.int(0xffffff).toString(16)}f`;
    if (!used.has(name)) {
      used.add(name);
      return name;
    }
  }
}

function identifierNames(ast) {
  const names = new Set();
  walk(ast, (n) => {
    if (n.type === "Identifier") names.add(n.name);
  });
  return names;
}

function inertStatement(r, kind, used) {
  if (kind === "var") return `var ${freshName(r, used)} = ${r.int(256)};`;
  if (kind === "function") return `function ${freshName(r, used)}() {}`;
  if (kind === "block") return "{}";
  if (kind === "empty") return ";";
  return "void 0;";
}

export function plan(r, ast, features) {
  const found = machinery(ast);
  const observesGlobals = features.includes("observes-global-names");
  const steps = [];
  const inertKinds = observesGlobals ? INERT_LOCAL : [...INERT_LOCAL, ...INERT_GLOBAL];
  if (r.chance(0.5)) steps.push({ kind: "inert", count: r.range(1, 4), forms: r.sample(inertKinds, r.range(1, inertKinds.length)) });
  if (found.decoders.length > 0 && r.chance(0.6)) {
    const kinds = observesGlobals ? ALIAS_KINDS.filter((k) => k === "let" || k === "const") : ALIAS_KINDS;
    steps.push({ kind: "alias", alias: r.pick(kinds), decoder: r.int(found.decoders.length), share: r.pick([0.5, 1]) });
  }
  if (found.decoders.length > 0 && r.chance(0.3)) steps.push({ kind: "rename-params", decoder: r.int(found.decoders.length) });
  if (r.chance(0.4)) steps.push({ kind: "reorder" });
  return steps;
}

function replacement(node, parent, key, text) {
  const shorthand = parent && parent.type === "Property" && parent.shorthand && key === "value";
  return { start: node.start, end: node.end, text: shorthand ? `${node.name}: ${text}` : text };
}

function topInsertAt(ast) {
  let at = 0;
  for (const statement of ast.body) {
    if (statement.type === "ExpressionStatement" && statement.directive !== undefined) at = statement.end;
    else break;
  }
  return at === 0 ? 0 : at;
}

function aliasDeclaration(kind, target, names) {
  const [a, b] = names;
  switch (kind) {
    case "var":
    case "let":
    case "const":
      return { decl: `${kind} ${a} = ${target};`, use: a };
    case "object":
      return { decl: `var ${a} = { ${b}: ${target} };`, use: `${a}.${b}` };
    case "wrapper":
      return { decl: `function ${a}(${b}, ${b}2) { return ${target}(${b}, ${b}2); }`, use: a };
    case "iife":
      return { decl: `var ${a} = (function () { return ${target}; })();`, use: a };
    case "function-expression":
      return { decl: `var ${a} = function (${b}, ${b}2) { return ${target}(${b}, ${b}2); };`, use: a };
    case "comma":
      return { decl: `var ${a} = (0, ${target});`, use: `(0, ${a})` };
    default:
      return { decl: `var ${a} = ${target}, ${b} = ${a};`, use: b };
  }
}

function applyAlias(r, step, ast, found, edits, used) {
  const decoder = found.decoders[step.decoder % found.decoders.length];
  const name = decoder.id.name;
  const names = [freshName(r, used), freshName(r, used)];
  const { decl, use } = aliasDeclaration(step.alias, name, names);
  edits.push({ start: topInsertAt(ast), end: topInsertAt(ast), text: `${decl}\n` });
  walk(ast, (n, parent, key) => {
    if (n.type !== "Identifier" || n.name !== name || isBindingSite(n, parent, key)) return;
    if (n.start >= decoder.start && n.end <= decoder.end) return;
    if (parent && parent.type === "AssignmentExpression" && key === "left") return;
    if (r.next() < step.share) edits.push(replacement(n, parent, key, use));
  });
}

function applyRename(r, step, found, edits, used) {
  const decoder = found.decoders[step.decoder % found.decoders.length];
  for (const param of decoder.params) {
    if (param.type !== "Identifier") continue;
    const fresh = `p${freshName(r, used).slice(3)}`;
    walk(decoder, (n, parent, key) => {
      if (n.type === "Identifier" && n.name === param.name && !isBindingSite(n, parent, key)) {
        edits.push(replacement(n, parent, key, fresh));
      }
    });
  }
}

function applyInert(r, step, ast, edits, used) {
  const slots = [topInsertAt(ast), ...ast.body.filter((s) => s.directive === undefined).map((s) => s.end)];
  for (let i = 0; i < step.count; i++) {
    const at = r.pick(slots);
    edits.push({ start: at, end: at, text: `\n${inertStatement(r, r.pick(step.forms), used)}\n` });
  }
}

function applyReorder(r, ast, source, edits) {
  const movable = ast.body.filter((s) => s.type === "FunctionDeclaration");
  if (movable.length === 0) return;
  const slots = [topInsertAt(ast), ...ast.body.filter((s) => s.directive === undefined && !movable.includes(s)).map((s) => s.end)];
  for (const fn of movable) {
    edits.push({ start: fn.start, end: fn.end, text: "" });
    const at = r.pick(slots);
    edits.push({ start: at, end: at, text: `\n${source.slice(fn.start, fn.end)}\n` });
  }
}

function overlaps(edits) {
  const spans = edits.filter((e) => e.end > e.start).sort((a, b) => a.start - b.start);
  for (let i = 1; i < spans.length; i++) if (spans[i].start < spans[i - 1].end) return true;
  const points = edits.filter((e) => e.end === e.start);
  return points.some((p) => spans.some((s) => p.start > s.start && p.start < s.end));
}

function applyEdits(source, edits) {
  const ordered = edits.map((e, i) => ({ ...e, i })).sort((a, b) => b.start - a.start || b.end - a.end || b.i - a.i);
  let out = source;
  for (const e of ordered) out = out.slice(0, e.start) + e.text + out.slice(e.end);
  return out;
}

export function apply(r, steps, parse, source) {
  let code = source;
  const applied = [];
  for (const step of steps) {
    const ast = parse(code);
    const found = machinery(ast);
    const used = identifierNames(ast);
    const edits = [];
    if (step.kind === "alias" && found.decoders.length > 0) applyAlias(r, step, ast, found, edits, used);
    else if (step.kind === "rename-params" && found.decoders.length > 0) applyRename(r, step, found, edits, used);
    else if (step.kind === "inert") applyInert(r, step, ast, edits, used);
    else if (step.kind === "reorder") applyReorder(r, ast, code, edits);
    if (edits.length === 0 || overlaps(edits)) continue;
    const next = applyEdits(code, edits);
    try {
      parse(next);
    } catch {
      continue;
    }
    code = next;
    applied.push(step);
  }
  return { code, applied };
}

export function envelope(code, env, sourceType) {
  if (sourceType === "module") return code;
  let out = code;
  if (env.iife) out = `(function () {\n${out}\n})();\n`;
  if (env.strict) out = `"use strict";\n${out}`;
  return out;
}
