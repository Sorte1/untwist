import { readFileSync } from "node:fs";
import { parse } from "@babel/parser";
import traverseModule from "@babel/traverse";

const traverse = traverseModule.default ?? traverseModule;

const MIN_ARRAY_STRINGS = 2;
const JSFUCK_MIN_RUN = 80;
const DECODING_CALLEES = new Set(["atob", "fromCharCode", "charCodeAt", "decodeURIComponent", "indexOf", "charAt"]);
const CORE_KEYS = [
  "stringArrays",
  "rotators",
  "decoderCalls",
  "arrayIndex",
  "cffSwitches",
  "opaquePredicates",
  "proxyObjects",
  "evalCalls",
  "functionCtors",
  "packer",
  "jsfuck",
  "jjencode",
  "aaencode",
];
const IDENT_KEYS = ["hexIdents", "escapedIdents", "confusableIdents"];

function parseSource(code) {
  const base = { allowReturnOutsideFunction: true, errorRecovery: true, plugins: ["jsx"] };
  try {
    return parse(code, { ...base, sourceType: "unambiguous" });
  } catch (first) {
    try {
      return parse(code, { ...base, sourceType: "script", plugins: [] });
    } catch {
      throw first;
    }
  }
}

function isStringSlot(element) {
  if (element?.type === "StringLiteral") return true;
  if (element?.type === "UnaryExpression") return element.operator === "void";
  return element?.type === "Identifier" && element.name === "undefined";
}

function isStringArray(node) {
  return (
    node?.type === "ArrayExpression" &&
    node.elements.length >= MIN_ARRAY_STRINGS &&
    node.elements.some((element) => element?.type === "StringLiteral") &&
    node.elements.every(isStringSlot)
  );
}

function functionBinding(path) {
  if (path.isFunctionDeclaration() && path.node.id) {
    return path.parentPath.scope.getBinding(path.node.id.name);
  }
  const parent = path.parentPath;
  if (parent?.isVariableDeclarator() && parent.node.init === path.node && parent.node.id.type === "Identifier") {
    return parent.scope.getBinding(parent.node.id.name);
  }
  if (parent?.isAssignmentExpression() && parent.node.right === path.node && parent.node.left.type === "Identifier") {
    return parent.scope.getBinding(parent.node.left.name);
  }
  return undefined;
}

function ownStatements(path) {
  const body = path.node.body;
  return body?.type === "BlockStatement" ? body.body : [];
}

function returnsStringArray(path) {
  return ownStatements(path).some(
    (statement) =>
      (statement.type === "ReturnStatement" && isStringArray(statement.argument)) ||
      (statement.type === "VariableDeclaration" && statement.declarations.some((d) => isStringArray(d.init))),
  );
}

function assignsOwnName(path, binding) {
  let found = false;
  path.traverse({
    AssignmentExpression(inner) {
      const left = inner.node.left;
      if (left.type === "Identifier" && left.name === binding.identifier.name && inner.scope.getBinding(left.name) === binding) {
        found = true;
        inner.stop();
      }
    },
  });
  return found;
}

function collectArrays(ast) {
  const arrays = new Map();
  traverse(ast, {
    Function(path) {
      if (path.node.params.length > 0 || !returnsStringArray(path)) return;
      const binding = functionBinding(path);
      if (!binding) return;
      arrays.set(binding, { kind: "function", path, selfAssigns: assignsOwnName(path, binding), rotated: false });
    },
    VariableDeclarator(path) {
      if (!isStringArray(path.node.init) || path.node.id.type !== "Identifier") return;
      const owner = path.getFunctionParent();
      if (owner && owner.node.params.length === 0 && returnsStringArray(owner)) return;
      const binding = path.scope.getBinding(path.node.id.name);
      if (binding) arrays.set(binding, { kind: "variable", path, selfAssigns: false, rotated: false });
    },
  });
  return arrays;
}

function memberName(node) {
  if (node.type !== "MemberExpression") return undefined;
  if (!node.computed && node.property.type === "Identifier") return node.property.name;
  if (node.computed && node.property.type === "StringLiteral") return node.property.value;
  return undefined;
}

function rotatorOf(refPath) {
  const call = refPath.findParent((p) => p.isCallExpression() && p.get("callee").isFunction());
  if (!call) return undefined;
  let found = false;
  call.get("callee").traverse({
    CallExpression(inner) {
      if (isPushShift(inner.node)) {
        found = true;
        inner.stop();
      }
    },
  });
  return found ? call : undefined;
}

function hasDecodingShape(path) {
  let found = false;
  const params = new Set(path.node.params.filter((p) => p.type === "Identifier").map((p) => p.name));
  path.traverse({
    BinaryExpression(inner) {
      const { operator, left, right } = inner.node;
      const literalOperand = left.type === "NumericLiteral" || right.type === "NumericLiteral" || right.type === "UnaryExpression" || right.type === "BinaryExpression";
      const paramOperand = (left.type === "Identifier" && params.has(left.name)) || (right.type === "Identifier" && params.has(right.name));
      if ((operator === "-" || operator === "+") && literalOperand && paramOperand) found = true;
    },
    CallExpression(inner) {
      const callee = inner.node.callee;
      const name = callee.type === "Identifier" ? callee.name : memberName(callee);
      if (name && DECODING_CALLEES.has(name)) found = true;
    },
  });
  return found;
}

function enclosingNamedFunction(refPath) {
  const fn = refPath.getFunctionParent();
  if (!fn || fn.node.params.length === 0) return undefined;
  const binding = functionBinding(fn);
  return binding ? { fn, binding } : undefined;
}

function isLiteralIndex(refPath) {
  const parent = refPath.parent;
  return parent.type === "MemberExpression" && parent.object === refPath.node && parent.computed && parent.property.type === "NumericLiteral";
}

function classifyArrayRefs(arrays) {
  const decoders = new Map();
  const rotators = new Set();
  let arrayIndex = 0;
  for (const [binding, info] of arrays) {
    const literalRefs = binding.referencePaths.filter(isLiteralIndex);
    if (info.kind === "variable" && binding.constantViolations.length === 0 && literalRefs.length > 0 && literalRefs.length === binding.referencePaths.length) {
      info.indexed = true;
      arrayIndex += literalRefs.length;
    }
    for (const ref of binding.referencePaths) {
      const rotator = rotatorOf(ref);
      if (rotator) {
        rotators.add(rotator.node);
        info.rotated = true;
        continue;
      }
      const owner = enclosingNamedFunction(ref);
      if (owner && !arrays.has(owner.binding)) {
        decoders.set(owner.binding, { fn: owner.fn, array: binding });
      }
    }
  }
  return { decoders, rotators, arrayIndex };
}

function qualifiedDecoders(arrays, decoders) {
  const result = new Map();
  for (const [binding, entry] of decoders) {
    const info = arrays.get(entry.array);
    const strongArray = info.selfAssigns || info.rotated;
    if (strongArray || assignsOwnName(entry.fn, binding) || hasDecodingShape(entry.fn)) {
      result.set(binding, entry.fn);
      info.decoded = true;
    }
  }
  return result;
}

function wrapperOf(callPath) {
  const ret = callPath.parentPath;
  if (ret?.isArrowFunctionExpression() && ret.node.body === callPath.node && ret.node.params.length > 0) return functionBinding(ret);
  if (!ret?.isReturnStatement()) return undefined;
  const fn = ret.getFunctionParent();
  if (!fn || fn.node.params.length === 0) return undefined;
  if (ownStatements(fn).length !== 1 && fn.node.body.type === "BlockStatement") return undefined;
  return functionBinding(fn);
}

function aliasTarget(refPath) {
  const parent = refPath.parentPath;
  if (parent.isVariableDeclarator() && parent.node.init === refPath.node && parent.node.id.type === "Identifier") {
    return parent.scope.getBinding(parent.node.id.name);
  }
  if (parent.isAssignmentExpression() && parent.node.right === refPath.node && parent.node.left.type === "Identifier") {
    return parent.scope.getBinding(parent.node.left.name);
  }
  return undefined;
}

function isLiteralArg(node) {
  if (!node) return false;
  if (node.type === "NumericLiteral" || node.type === "StringLiteral") return true;
  if (node.type === "UnaryExpression") return isLiteralArg(node.argument);
  if (node.type === "BinaryExpression") return isLiteralArg(node.left) && isLiteralArg(node.right);
  return false;
}

function insideAny(path, nodes) {
  return Boolean(path.findParent((p) => nodes.has(p.node)));
}

function expandAliases(decoders, rotators) {
  const machinery = new Set([...decoders.values()].map((fn) => fn.node));
  const members = new Set(decoders.keys());
  const queue = [...members];
  const calls = [];
  let aliases = 0;
  while (queue.length > 0) {
    const binding = queue.pop();
    for (const ref of binding.referencePaths) {
      const target = ref.parentPath.isCallExpression() && ref.parent.callee === ref.node ? wrapperOf(ref.parentPath) : aliasTarget(ref);
      if (target && !members.has(target)) {
        members.add(target);
        queue.push(target);
        aliases += 1;
        if (target.path.isFunction() || target.path.isVariableDeclarator()) machinery.add(target.path.isFunction() ? target.path.node : target.path.node.init);
        continue;
      }
      if (ref.parentPath.isCallExpression() && ref.parent.callee === ref.node && !members.has(target)) calls.push(ref.parentPath);
    }
  }
  const live = calls.filter((call) => !insideAny(call, machinery) && !insideAny(call, rotators));
  return { aliases, decoderCalls: live.length, decoderCallsLiteral: live.filter((c) => isLiteralArg(c.node.arguments[0])).length };
}

function isParamRef(node, params) {
  return node?.type === "Identifier" && params.includes(node.name);
}

function isProxyFunction(node) {
  if (node?.type !== "FunctionExpression" && node?.type !== "ArrowFunctionExpression") return false;
  const params = node.params.map((p) => (p.type === "Identifier" ? p.name : undefined));
  if (params.length === 0 || params.includes(undefined)) return false;
  const body = node.body.type === "BlockStatement" ? (node.body.body.length === 1 && node.body.body[0].type === "ReturnStatement" ? node.body.body[0].argument : undefined) : node.body;
  if (!body) return false;
  if (body.type === "BinaryExpression" || body.type === "LogicalExpression") {
    return params.length === 2 && isParamRef(body.left, params) && isParamRef(body.right, params);
  }
  if (body.type === "CallExpression") {
    return isParamRef(body.callee, params.slice(0, 1)) && body.arguments.every((a, i) => a.type === "Identifier" && a.name === params[i + 1]);
  }
  return false;
}

function proxyCount(node) {
  const props = node.properties;
  if (props.length === 0) return 0;
  let proxies = 0;
  for (const prop of props) {
    if (prop.type !== "ObjectProperty" || prop.computed) return 0;
    if (isProxyFunction(prop.value)) proxies += 1;
  }
  return proxies;
}

function isGlobalCallee(path, name) {
  const callee = path.node.callee;
  return callee.type === "Identifier" && callee.name === name && !path.scope.getBinding(name);
}

function isPacker(path) {
  const inner = path.node.arguments[0];
  if (inner?.type !== "CallExpression" || inner.callee.type !== "FunctionExpression" || inner.callee.params.length < 4) return false;
  return inner.arguments.some((arg) => arg.type === "CallExpression" && memberName(arg.callee) === "split");
}

function isOpaqueTest(test) {
  return test?.type === "BinaryExpression" && ["===", "!==", "==", "!="].includes(test.operator) && test.left.type === "StringLiteral" && test.right.type === "StringLiteral";
}

function isDispatcher(path) {
  const discriminant = path.node.discriminant;
  if (discriminant.type !== "MemberExpression" || !discriminant.computed || discriminant.property.type !== "UpdateExpression") return false;
  const loop = path.parentPath?.parentPath;
  return Boolean(loop && (loop.isLoop() || path.parentPath.isLoop()));
}

function identifierClass(code, binding) {
  const name = binding.identifier.name;
  const raw = code.slice(binding.identifier.start, binding.identifier.end);
  return {
    hex: /^_0x[0-9a-f]+$/i.test(name) || /^a0_0x[0-9a-f]+$/i.test(name),
    escaped: raw.includes("\\u"),
    confusable: /[^\x00-\x7f]/.test(name),
  };
}

function isPushShift(node) {
  if (node.type !== "CallExpression" || memberName(node.callee) !== "push" || node.arguments.length !== 1) return false;
  const inner = node.arguments[0];
  return inner.type === "CallExpression" && memberName(inner.callee) === "shift" && inner.callee.object.type === "Identifier" && node.callee.object.type === "Identifier" && inner.callee.object.name === node.callee.object.name;
}

function scanStructure(ast, code, counts, rotators) {
  const scopes = new Set();
  const proxyOwners = new Set();
  traverse(ast, {
    Scopable(path) {
      if (scopes.has(path.scope)) return;
      scopes.add(path.scope);
      for (const binding of Object.values(path.scope.bindings)) {
        const kind = identifierClass(code, binding);
        if (kind.hex) counts.hexIdents += 1;
        if (kind.escaped) counts.escapedIdents += 1;
        if (kind.confusable) counts.confusableIdents += 1;
      }
    },
    "IfStatement|ConditionalExpression"(path) {
      if (isOpaqueTest(path.node.test)) counts.opaquePredicates += 1;
    },
    SwitchStatement(path) {
      if (isDispatcher(path)) counts.cffSwitches += 1;
    },
    AssignmentExpression(path) {
      const left = path.node.left;
      if (left.type !== "MemberExpression" || left.object.type !== "Identifier" || !isProxyFunction(path.node.right)) return;
      const owner = path.scope.getBinding(left.object.name) ?? left.object.name;
      if (!proxyOwners.has(owner)) {
        proxyOwners.add(owner);
        counts.proxyObjects += 1;
      }
      counts.proxyProps += 1;
    },
    ObjectExpression(path) {
      const proxies = proxyCount(path.node);
      if (proxies > 0) {
        counts.proxyObjects += 1;
        counts.proxyProps += proxies;
      }
    },
    CallExpression(path) {
      if (isPushShift(path.node)) {
        const iife = path.findParent((p) => p.isCallExpression() && p.get("callee").isFunction());
        if (iife) rotators.add(iife.node);
      }
      if (isGlobalCallee(path, "eval")) {
        counts.evalCalls += 1;
        if (isPacker(path)) counts.packer += 1;
      }
      if (isGlobalCallee(path, "Function")) counts.functionCtors += 1;
      if (memberName(path.node.callee) === "constructor" && path.node.arguments[0]?.type === "StringLiteral") counts.functionCtors += 1;
    },
    NewExpression(path) {
      if (isGlobalCallee(path, "Function")) counts.functionCtors += 1;
    },
  });
}

function scanSignatures(code, counts) {
  const jsfuckRun = new RegExp(`[\\[\\]()!+]{${JSFUCK_MIN_RUN},}`, "g");
  counts.jsfuck = (code.match(jsfuckRun) ?? []).length;
  counts.jjencode = (code.match(/([\w$]+)=~\[\];\s*\1=\{___:\+\+\1/g) ?? []).length;
  counts.aaencode = (code.match(/ﾟωﾟﾉ\s*=/g) ?? []).length;
}

function emptyCounts() {
  const keys = [...CORE_KEYS, ...IDENT_KEYS, "decoders", "decoderAliases", "decoderCallsLiteral", "proxyProps"];
  return Object.fromEntries(keys.map((key) => [key, 0]));
}

const MODULE_BANNER = /^\/\/ ==== .* ====$/m;

function mergeCounts(parts) {
  const merged = {};
  for (const part of parts) {
    for (const [key, value] of Object.entries(part)) {
      if (typeof value === "number") merged[key] = (merged[key] ?? 0) + value;
      else if (value !== null && value !== undefined && merged[key] === undefined) merged[key] = value;
    }
  }
  if (parts.some((part) => part.core === null)) merged.core = null;
  return { ...merged, modules: parts.length };
}

export function measure(code) {
  if (!MODULE_BANNER.test(code)) return measureModule(code);
  return mergeCounts(code.split(new RegExp(MODULE_BANNER.source, "m")).map(measureModule));
}

function measureModule(code) {
  const counts = emptyCounts();
  scanSignatures(code, counts);
  let ast;
  try {
    ast = parseSource(code);
  } catch (error) {
    return { ...counts, parseError: String(error.message).slice(0, 200), core: null, idents: null };
  }
  const arrays = collectArrays(ast);
  const { decoders, rotators, arrayIndex } = classifyArrayRefs(arrays);
  const live = qualifiedDecoders(arrays, decoders);
  const aliases = expandAliases(live, rotators);
  counts.decoders = live.size;
  counts.decoderAliases = aliases.aliases;
  counts.decoderCalls = aliases.decoderCalls;
  counts.decoderCallsLiteral = aliases.decoderCallsLiteral;
  scanStructure(ast, code, counts, rotators);
  counts.rotators = rotators.size;
  counts.arrayIndex = arrayIndex;
  counts.stringArrays = [...arrays.values()].filter((info) => info.decoded || info.rotated || info.indexed || info.selfAssigns).length;
  const sum = (keys) => keys.reduce((total, key) => total + counts[key], 0);
  return { ...counts, parseErrors: ast.errors?.length ?? 0, core: sum(CORE_KEYS), idents: sum(IDENT_KEYS) };
}

function main(argv) {
  const files = argv[0] === "--list" ? readFileSync(argv[1], "utf8").split("\n").filter(Boolean) : argv;
  for (const file of files) {
    try {
      const code = readFileSync(file, "utf8");
      process.stdout.write(`${JSON.stringify({ file, bytes: Buffer.byteLength(code), ...measure(code) })}\n`);
    } catch (error) {
      process.stdout.write(`${JSON.stringify({ file, error: String(error.message).slice(0, 200) })}\n`);
    }
  }
}

if (import.meta.url === `file://${process.argv[1]}`) main(process.argv.slice(2));
