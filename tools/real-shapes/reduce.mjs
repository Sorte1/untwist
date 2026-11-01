import { spawnSync } from "node:child_process";
import { mkdtempSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const require = createRequire(join(HERE, "../obfuscated/package.json"));
const acorn = require("acorn");

const CELLS = [
  ["safe-none-stdout", []],
  ["unsafe-none-stdout", ["--unsafe"]],
  ["unsafe-simple-stdout", ["--unsafe", "--rename", "simple"]],
  ["unsafe-none-dir", ["--unsafe", "DIR"]],
];
const MACHINERY = new Set([
  "find-string-array",
  "find-array-rotator",
  "find-decoders",
  "inline-decoded-strings",
  "inline-decoder-wrappers",
  "remove-decoder-machinery",
  "unstamp-remaining-machinery",
  "deobfuscate",
  "pipeline",
  "debug-protection",
  "self-defending",
  "strip-shell",
  "unpack",
]);
const IGNORED = [/simple string array not inlined/, /string array found: \d+ entries/];
const DECODER_CALL = /(?<![\w$.])[A-Za-z_$\u0080-￿][\w$\u0080-￿]*\((?:0x[0-9a-fA-F]+|\d+)(?:,\s*(?:'[^'\n]{0,8}'|"[^"\n]{0,8}"))?\)/g;
const STRING_ARRAY = /\[(?:'(?:[^'\\\n]|\\.)*'|"(?:[^"\\\n]|\\.)*")(?:,\s*(?:'(?:[^'\\\n]|\\.)*'|"(?:[^"\\\n]|\\.)*")){9,}\]/;
const MIN_MODULE_TABLE = 8;
const GLOBAL_MEMBERS = new Set(["exports", "prototype", "call", "apply", "bind", "push", "shift", "length", "constructor", "toString", "defineProperty", "hasOwnProperty", "create", "keys", "parseInt", "fromCharCode", "charCodeAt", "indexOf", "slice", "split", "join", "replace", "test", "exec", "search", "concat", "default"]);

function parseArgs(argv) {
  const options = { budget: "600", timeout: "600" };
  for (let i = 0; i < argv.length; i += 2) options[argv[i].replace(/^--/, "")] = argv[i + 1];
  if (!options.in || !options.out || !options.untwist) {
    throw new Error("usage: reduce.mjs --in <file> --out <file> --untwist <bin> [--budget tests]");
  }
  return { ...options, budget: Number(options.budget) };
}

function classOf(line) {
  const body = line.replace(/^note: /, "");
  const at = body.indexOf(": ");
  const pass = at > 0 && !body.slice(0, at).includes(" ") ? body.slice(0, at) : "cli";
  let message = at > 0 ? body.slice(at + 2) : body;
  message = message.replace(/^find-string-array: /, "");
  const head = message.split(/[:,;(]/)[0];
  return { pass, key: `${pass}: ${head.replace(/\d+/g, "N").trim()}` };
}

function count(regex, text) {
  return (text.match(regex) ?? []).length;
}

function readOutput(dir) {
  const parts = [];
  const walk = (d) => {
    for (const name of readdirSync(d)) {
      const path = join(d, name);
      if (statSync(path).isDirectory()) walk(path);
      else if (name.endsWith(".js")) parts.push(readFileSync(path, "utf8"));
    }
  };
  walk(dir);
  return parts.join("\n");
}

function fingerprint(options, file, work) {
  const input = readFileSync(file, "utf8");
  const inputCalls = count(DECODER_CALL, input);
  const cells = [];
  for (const [label, flags] of CELLS) {
    const out = join(work, `${label}`);
    const args = ["--report", "text", ...flags.filter((f) => f !== "DIR"), ...(flags.includes("DIR") ? ["-f", "-o", out] : []), file];
    const result = spawnSync("timeout", [options.timeout, options.untwist, ...args], { encoding: "utf8", maxBuffer: 1 << 30 });
    const output = flags.includes("DIR") ? safeRead(out) : result.stdout ?? "";
    const notes = (result.stderr ?? "")
      .split("\n")
      .filter((line) => line.startsWith("note: ") && !IGNORED.some((re) => re.test(line)))
      .map(classOf)
      .filter((c) => MACHINERY.has(c.pass))
      .map((c) => c.key);
    const calls = count(DECODER_CALL, output);
    const decoded = calls === 0 ? "all" : calls * 2 < inputCalls ? "most" : calls < inputCalls ? "some" : "none";
    cells.push({
      label,
      exit: result.status,
      bundle: (/^bundle: (\w+)/m.exec(result.stderr ?? "") ?? [])[1] ?? "none",
      array: STRING_ARRAY.test(output),
      decoded,
      notes: [...new Set(notes)].sort(),
    });
  }
  return JSON.stringify(cells);
}

function safeRead(dir) {
  try {
    return readOutput(dir);
  } catch {
    return "";
  }
}

function parse(text) {
  const options = { ecmaVersion: "latest", allowReturnOutsideFunction: true, allowHashBang: true };
  let ast;
  try {
    ast = acorn.parse(text, { ...options, sourceType: "script" });
  } catch {
    ast = acorn.parse(text, { ...options, sourceType: "module", allowReturnOutsideFunction: false });
  }
  ast.__text = text;
  return ast;
}

function children(node) {
  const out = [];
  for (const value of Object.values(node)) {
    if (Array.isArray(value)) out.push(...value.filter((v) => v && typeof v.type === "string"));
    else if (value && typeof value.type === "string") out.push(value);
  }
  return out;
}

function visit(node, fn, depth = 0) {
  fn(node, depth);
  for (const child of children(node)) visit(child, fn, depth + 1);
}

function isFunction(node) {
  return node && (node.type === "FunctionExpression" || node.type === "ArrowFunctionExpression");
}

function moduleBodies(ast) {
  let best = [];
  visit(ast, (node) => {
    const items = node.type === "ArrayExpression" ? node.elements : node.type === "ObjectExpression" ? node.properties.map((p) => p.value) : [];
    const functions = items.filter(isFunction);
    if (functions.length >= MIN_MODULE_TABLE && functions.length * 2 >= items.length && functions.length > best.length) best = functions;
  });
  return best.filter((f) => f.body.type === "BlockStatement" && f.body.body.length > 0).map((f) => [f.body.start, f.body.end, "{}"]);
}

function statementLists(ast) {
  const lists = [];
  visit(ast, (node, depth) => {
    if ((node.type === "Program" || node.type === "BlockStatement") && node.body.length > 1) lists.push({ depth, items: node.body });
  });
  return lists.sort((a, b) => a.depth - b.depth || b.items.length - a.items.length);
}

function textEntries(ast) {
  return biggestStringArray(ast).filter(([start], _i, all) => !/^['"]\d/.test(ast.__text.slice(start, start + 2)));
}

function nonChecksumEntries(ast) {
  return biggestStringArray(ast).filter(([start, end]) => !/^['"]\d+[A-Za-z_]{4,}['"]$/.test(ast.__text.slice(start, end)));
}

function regexLiterals(ast) {
  const edits = [];
  visit(ast, (node) => {
    if (node.type === "Literal" && node.regex && node.raw !== "/x/") edits.push([node.start, node.end, "/x/"]);
  });
  return edits;
}

function propertyNames(ast) {
  const edits = [];
  let n = 0;
  visit(ast, (node) => {
    if (node.type === "MemberExpression" && !node.computed && node.property.type === "Identifier" && /^[A-Za-z_$][\w$]{3,}$/.test(node.property.name) && !GLOBAL_MEMBERS.has(node.property.name)) {
      edits.push([node.property.start, node.property.end, `p${(n += 1)}`]);
    }
  });
  return edits;
}

const KEPT_WORDS = new Set([...GLOBAL_MEMBERS, "Object", "Math", "JSON", "Symbol", "Promise", "window", "eval", "Date", "Function", "Array", "String", "Number", "Boolean", "RegExp", "Error", "TypeError", "Float32Array", "WeakMap", "Map", "Set", "Reflect", "globalThis", "self", "undefined", "arguments", "require", "module", "__webpack_require__", "__webpack_exports__", "__esModule", "anonymous", "async", "await", "console", "setTimeout", "setInterval", "document", "navigator", "location"]);

function wordsAndStrings(ast) {
  const array = new Set(biggestStringArray(ast).map(([start]) => start));
  const names = new Map();
  const edits = [];
  visit(ast, (node) => {
    if (node.type === "Identifier" && /^[A-Za-z_$][\w$]{2,}$/.test(node.name) && !KEPT_WORDS.has(node.name)) {
      if (!names.has(node.name)) names.set(node.name, `w${names.size + 1}`);
      edits.push([node.start, node.end, names.get(node.name)]);
    } else if (node.type === "Literal" && typeof node.value === "string" && node.value.length > 1 && !array.has(node.start) && !KEPT_WORDS.has(node.value)) {
      edits.push([node.start, node.end, `${node.raw[0]}s${node.raw[0]}`]);
    } else if (node.type === "TemplateElement" && node.value.raw.length > 0) {
      edits.push([node.start, node.end, ""]);
    }
  });
  return [edits];
}

function biggestStringArray(ast) {
  let best;
  visit(ast, (node) => {
    if (node.type !== "ArrayExpression" || node.elements.length < 10) return;
    if (!node.elements.every((e) => e && e.type === "Literal" && typeof e.value === "string")) return;
    if (!best || node.elements.length > best.elements.length) best = node;
  });
  return best ? best.elements.filter((e) => e.end - e.start > 2).map((e) => [e.start, e.end, e.raw[0] + e.raw[0]]) : [];
}

function otherStrings(ast) {
  const array = new Set();
  visit(ast, (node) => {
    if (node.type === "ArrayExpression" && node.elements.length >= 10) for (const e of node.elements) if (e) array.add(e.start);
  });
  const edits = [];
  visit(ast, (node) => {
    if (node.type === "Literal" && typeof node.value === "string" && node.value.length > 1 && !array.has(node.start)) edits.push([node.start, node.end, `${node.raw[0]}s${node.raw[0]}`]);
  });
  return edits;
}

function apply(text, edits) {
  const sorted = [...edits].sort((a, b) => b[0] - a[0]);
  let out = text;
  let floor = Infinity;
  for (const [start, end, replacement] of sorted) {
    if (end > floor) continue;
    out = out.slice(0, start) + replacement + out.slice(end);
    floor = start;
  }
  return out;
}

class Reducer {
  constructor(options) {
    this.options = options;
    this.work = mkdtempSync(join(resolve(options.scratch ?? dirname(options.out)), "reduce-"));
    this.tests = 0;
    this.limit = options.budget;
    this.text = readFileSync(options.in, "utf8");
    this.target = this.check(this.text);
    this.log(`target ${this.target}`);
  }

  log(message) {
    process.stdout.write(`[${this.tests}] ${message}\n`);
  }

  check(text) {
    const file = join(this.work, "candidate.js");
    writeFileSync(file, text);
    try {
      parse(text);
    } catch {
      return "unparsable";
    }
    this.tests += 1;
    return fingerprint(this.options, file, this.work);
  }

  keeps(text) {
    if (this.tests >= this.limit) return false;
    return this.check(text) === this.target;
  }

  ddmin(label, editsOf) {
    let edits = editsOf(parse(this.text));
    let chunk = edits.length;
    this.log(`${label}: ${edits.length} candidate edit(s), ${this.text.length} bytes`);
    while (chunk >= 1 && edits.length > 0 && this.tests < this.limit) {
      let progressed = false;
      for (let at = 0; at < edits.length && this.tests < this.limit; at += chunk) {
        const slice = edits.slice(at, at + chunk);
        const candidate = apply(this.text, slice);
        if (candidate !== this.text && this.keeps(candidate)) {
          this.text = candidate;
          writeFileSync(this.options.out, this.text);
          this.log(`${label}: applied ${slice.length}, now ${this.text.length} bytes`);
          edits = editsOf(parse(this.text));
          progressed = true;
          break;
        }
      }
      if (!progressed) chunk = Math.floor(chunk / 2);
      else chunk = Math.min(chunk, edits.length);
    }
  }

  statements() {
    let before;
    do {
      before = this.text.length;
      this.ddmin("statements", (ast) => statementLists(ast).flatMap((list) => list.items.map((s) => [s.start, s.end, ";"])));
    } while (this.text.length < before && this.tests < this.limit);
  }
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const reducer = new Reducer(options);
  writeFileSync(options.out, reducer.text);
  const phase = (share, work) => {
    reducer.limit = Math.min(options.budget, reducer.tests + Math.ceil(options.budget * share));
    work();
  };
  if (options.words) {
    const ast = parse(reducer.text);
    const all = wordsAndStrings(ast)[0];
    const edits = options.words === "strings" ? all.filter(([, , rep]) => !/^w\d+$/.test(rep)) : all;
    const candidate = apply(reducer.text, edits);
    const kept = reducer.keeps(candidate);
    if (kept) {
      reducer.text = candidate;
      writeFileSync(options.out, candidate);
    }
    reducer.log(`words and strings: ${kept ? "renamed" : "fingerprint changed, kept as is"}`);
    reducer.limit = reducer.tests;
  }
  if (options.stringsAll) {
    phase(1, () => reducer.ddmin("all strings", (ast) => wordsAndStrings(ast)[0].filter(([, , rep]) => !/^w\d+$/.test(rep))));
  }
  if (options.sanitize) {
    phase(0.15, () => reducer.ddmin("non-checksum entries", nonChecksumEntries));
    phase(0.1, () => reducer.ddmin("regex", regexLiterals));
    phase(0.15, () => reducer.ddmin("strings", otherStrings));
    phase(0.2, () => reducer.ddmin("property names", propertyNames));
  }
  phase(0.1, () => reducer.ddmin("modules", moduleBodies));
  phase(0.05, () => reducer.ddmin("string array text", textEntries));
  phase(0.25, () => reducer.ddmin("string array", biggestStringArray));
  phase(0.15, () => reducer.ddmin("strings", otherStrings));
  phase(1, () => reducer.statements());
  const final = reducer.check(reducer.text) === reducer.target;
  reducer.log(`done: ${reducer.text.length} bytes, ${reducer.tests} tests, fingerprint ${final ? "kept" : "LOST"}`);
  if (!final) process.exit(1);
}

main();
