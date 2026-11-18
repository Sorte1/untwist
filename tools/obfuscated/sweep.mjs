import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { appendFileSync, existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { cpus, homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { Worker, isMainThread, parentPort, workerData } from "node:worker_threads";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../..");
const SOURCES = join(HERE, "sources");
const require = createRequire(join(HERE, "package.json"));
const PACKAGE = "javascript-obfuscator-5";
const PACKAGE_DIR = dirname(require.resolve(`${PACKAGE}/package.json`));
const TYPINGS = join(PACKAGE_DIR, "typings/src");

const PROGRAMS = ["empty-log", "greeting", "functions", "objects", "classes", "control", "strings", "mixed"];
const INLINE_PROGRAMS = { "console-test": 'console.log("test");\n' };
const NO_CODE_EFFECT = ["advertisement", "config", "exclude", "identifierNamesCache", "inputFileName", "log", "optionsPreset", "seed", "sourceMap", "sourceMapBaseUrl", "sourceMapFileName", "sourceMapMode", "sourceMapSourcesMode"];
const THRESHOLD_DOMAIN = [0, 0.5, 1];
const SYLLABLES = "bcdfghklmnprstvz".split("").flatMap((consonant) => "aeiou".split("").map((vowel) => consonant + vowel));
const DICTIONARY = SYLLABLES.flatMap((first) => SYLLABLES.map((second) => first + second + "n"));
const DICTIONARY_TOKEN = "@syllable-words";
const NUMBER_DOMAINS = {
  debugProtectionInterval: [0, 4000],
  splitStringsChunkLength: [1, 2, 10],
  stringArrayWrappersCount: [0, 1, 2, 5],
  stringArrayWrappersParametersMaxCount: [2, 5],
};
const LIST_DOMAINS = {
  domainLock: [[], ["example.com"]],
  forceTransformStrings: [[], ["^l", "s$"]],
  identifiersDictionary: [[DICTIONARY_TOKEN]],
  reservedNames: [[], ["^keep"]],
  reservedStrings: [[], ["^keep", "^done"]],
};
const STRING_DOMAINS = {
  domainLockRedirectUrl: ["about:blank", "https://example.com/locked"],
  identifiersPrefix: ["", "obf"],
};
const SUBSET_MIN = { stringArrayEncoding: 0, stringArrayIndexesType: 1 };
const PRESETS = ["default", "low-obfuscation", "medium-obfuscation", "high-obfuscation"];
const PREMISE_NOTE = /^(inline-decoded-strings|remove-decoder-machinery|inline-decoder-wrappers|self-defending|debug-protection|console|domain-lock|pipeline): .*(global object|code from text|standard built-in|replace a built-in|patched|outside-code premise|would hang where the input runs)/;
const MACHINERY_PASSES = ["find-string-array", "find-decoders", "find-array-rotator", "inline-decoder-wrappers", "remove-decoder-machinery", "self-defending", "debug-protection", "console"];

function readTyping(path) {
  return readFileSync(join(TYPINGS, `${path}.d.ts`), "utf8");
}

function importsOf(text) {
  const map = {};
  for (const match of text.matchAll(/import \{ (\w+) \} from '([^']+)';/g)) map[match[1]] = match[2];
  return map;
}

function enumValues(fromFile, name) {
  const path = resolve(dirname(join(TYPINGS, fromFile)), importsOf(readTyping(fromFile))[name]);
  const text = readFileSync(`${path}.d.ts`, "utf8");
  const values = [...text.matchAll(/:\s*'([^']+)';/g)].map((match) => match[1]);
  if (values.length === 0) throw new Error(`no values for enum ${name}`);
  return values;
}

function aliasValues(fromFile, alias) {
  const aliasFile = join(dirname(fromFile), importsOf(readTyping(fromFile))[alias]);
  const text = readTyping(aliasFile);
  const inner = text.match(/TTypeFromEnum<typeof (\w+)>/);
  if (!inner) throw new Error(`alias ${alias} is not an enum`);
  return enumValues(aliasFile, inner[1]);
}

function subsets(values, min) {
  const out = [];
  for (let mask = 0; mask < 1 << values.length; mask++) {
    const picked = values.filter((_, index) => mask & (1 << index));
    if (picked.length >= min) out.push(picked);
  }
  return out;
}

function domainOf(name, type) {
  const file = "interfaces/options/IOptions";
  if (type === "boolean") return [false, true];
  if (type === "number") {
    if (name.endsWith("Threshold")) return THRESHOLD_DOMAIN;
    if (NUMBER_DOMAINS[name]) return NUMBER_DOMAINS[name];
  }
  if (type === "string[]" && LIST_DOMAINS[name]) return LIST_DOMAINS[name];
  if (type === "string" && STRING_DOMAINS[name]) return STRING_DOMAINS[name];
  const enumType = type.match(/^TTypeFromEnum<typeof (\w+)>$/);
  if (enumType) return enumValues(file, enumType[1]);
  const listAlias = type.match(/^(T\w+)\[\]$/);
  if (listAlias && name in SUBSET_MIN) return subsets(aliasValues(file, listAlias[1]), SUBSET_MIN[name]);
  const alias = type.match(/^(T\w+)$/);
  if (alias) return aliasValues(file, alias[1]);
  throw new Error(`no value domain for option ${name}: ${type}; add one to sweep.mjs`);
}

function optionDomains() {
  const text = readTyping("interfaces/options/IOptions");
  const domains = {};
  for (const match of text.matchAll(/readonly (\w+): ([^;]+);/g)) {
    const [, name, type] = match;
    if (NO_CODE_EFFECT.includes(name)) continue;
    domains[name] = domainOf(name, type.trim());
  }
  return domains;
}

function rng(seed) {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const pick = (random, list) => list[Math.floor(random() * list.length)];

function pairKey(i, a, j, b) {
  return i < j ? `${i}:${a}|${j}:${b}` : `${j}:${b}|${i}:${a}`;
}

function uncoveredPairs(sizes) {
  const set = new Set();
  for (let i = 0; i < sizes.length; i++) {
    for (let j = i + 1; j < sizes.length; j++) {
      for (let a = 0; a < sizes[i]; a++) for (let b = 0; b < sizes[j]; b++) set.add(pairKey(i, a, j, b));
    }
  }
  return set;
}

function withDictionary(options) {
  return options.identifiersDictionary?.[0] === DICTIONARY_TOKEN ? { ...options, identifiersDictionary: DICTIONARY } : options;
}

function probeValid(options) {
  const Obfuscator = require(PACKAGE);
  try {
    Obfuscator.obfuscate("", withDictionary({ identifiersDictionary: [DICTIONARY_TOKEN], ...options }));
    return true;
  } catch {
    return false;
  }
}

function invalidPairs(names, domains) {
  const invalid = new Set();
  for (let i = 0; i < names.length; i++) {
    for (let j = i + 1; j < names.length; j++) {
      domains[names[i]].forEach((a, ai) => {
        domains[names[j]].forEach((b, bi) => {
          if (!probeValid({ [names[i]]: a, [names[j]]: b })) invalid.add(pairKey(i, ai, j, bi));
        });
      });
    }
  }
  return invalid;
}

function fits(row, index, value, invalid) {
  for (let other = 0; other < row.length; other++) {
    if (other !== index && row[other] !== undefined && invalid.has(pairKey(index, value, other, row[other]))) return false;
  }
  return true;
}

function gain(row, index, value, open) {
  let count = 0;
  for (let other = 0; other < row.length; other++) {
    if (other !== index && row[other] !== undefined && open.has(pairKey(index, value, other, row[other]))) count++;
  }
  return count;
}

function candidateRow(sizes, open, invalid, random) {
  const row = new Array(sizes.length);
  const seedPair = pick(random, [...open].slice(0, 64)).split("|").map((part) => part.split(":").map(Number));
  for (const [index, value] of seedPair) row[index] = value;
  const order = sizes.map((_, index) => index).filter((index) => row[index] === undefined).sort(() => random() - 0.5);
  for (const index of order) {
    let best = [];
    let bestGain = -1;
    for (let value = 0; value < sizes[index]; value++) {
      if (!fits(row, index, value, invalid)) continue;
      const g = gain(row, index, value, open);
      if (g > bestGain) [best, bestGain] = [[value], g];
      else if (g === bestGain) best.push(value);
    }
    row[index] = best.length > 0 ? pick(random, best) : 0;
  }
  return row;
}

function covered(row, open) {
  let count = 0;
  for (let i = 0; i < row.length; i++) for (let j = i + 1; j < row.length; j++) if (open.has(pairKey(i, row[i], j, row[j]))) count++;
  return count;
}

function rowOptions(names, domains, row) {
  return Object.fromEntries(names.map((name, index) => [name, domains[name][row[index]]]));
}

function pairwise(domains, invalid, seed) {
  const names = Object.keys(domains);
  const sizes = names.map((name) => domains[name].length);
  const open = uncoveredPairs(sizes);
  for (const key of invalid) open.delete(key);
  const random = rng(seed);
  const rows = [];
  while (open.size > 0) {
    let best = null;
    let bestCount = -1;
    for (let attempt = 0; attempt < 24; attempt++) {
      const row = candidateRow(sizes, open, invalid, random);
      const count = covered(row, open);
      if (count > bestCount) [best, bestCount] = [row, count];
    }
    for (let i = 0; i < best.length; i++) for (let j = i + 1; j < best.length; j++) open.delete(pairKey(i, best[i], j, best[j]));
    rows.push(rowOptions(names, domains, best));
  }
  return { rows, pairs: uncoveredPairs(sizes).size - invalid.size };
}

function randomRows(domains, invalid, count, seed) {
  const names = Object.keys(domains);
  const random = rng(seed ^ 0x5eed);
  const rows = [];
  while (rows.length < count) {
    const row = [];
    names.forEach((name, index) => {
      const choices = domains[name].map((_, value) => value).filter((value) => fits(row, index, value, invalid));
      row[index] = pick(random, choices.length > 0 ? choices : [0]);
    });
    rows.push(rowOptions(names, domains, row));
  }
  return rows;
}

function combos(domains, randomCount, seed) {
  const list = [];
  const invalid = invalidPairs(Object.keys(domains), domains);
  const covering = pairwise(domains, invalid, seed);
  covering.rows.forEach((options, index) => list.push({ id: `pw${String(index).padStart(3, "0")}`, options }));
  for (const preset of PRESETS) list.push({ id: `preset-${preset.replace("-obfuscation", "")}`, options: { optionsPreset: preset } });
  randomRows(domains, invalid, randomCount, seed).forEach((options, index) => list.push({ id: `rnd${String(index).padStart(3, "0")}`, options }));
  return { list, pairs: covering.pairs, invalidPairs: invalid.size, pairwiseRows: covering.rows.length };
}

function programs() {
  const out = {};
  for (const name of PROGRAMS) out[name] = readFileSync(join(SOURCES, `${name}.js`), "utf8");
  return { ...out, ...INLINE_PROGRAMS };
}

function plan(config) {
  const domains = optionDomains();
  const { list, pairs, invalidPairs: invalidCount, pairwiseRows } = combos(domains, config.random, config.seed);
  const random = rng(config.seed ^ 0x1a7e);
  const sources = Object.keys(programs());
  const cases = [];
  list.forEach((combo, index) => {
    const other = () => {
      let at = Math.floor(random() * list.length);
      if (at === index) at = (at + 1) % list.length;
      return list[at];
    };
    const mixed2 = [combo, other()];
    const mixed3 = [combo, mixed2[1], other()];
    const chains = [
      { tag: "n1", layers: [combo] },
      { tag: "n2", layers: [combo, combo] },
      { tag: "n3", layers: [combo, combo, combo] },
      { tag: "m2", layers: mixed2 },
      { tag: "m3", layers: mixed3 },
    ];
    sources.forEach((source, sourceIndex) => {
      for (const chain of chains) {
        if (chain.tag !== "n1" && (sourceIndex + index) % config.layerStride !== 0) continue;
        cases.push({ id: `${combo.id}-${chain.tag}--${source}`, combo: combo.id, chain: chain.tag, depth: chain.layers.length, mixed: chain.tag.startsWith("m"), source, layers: chain.layers.map((layer) => ({ id: layer.id, options: layer.options })) });
      }
    });
  });
  return { domains, combos: list, cases, pairs, invalidPairs: invalidCount, pairwiseRows };
}

const sha = (text) => createHash("sha256").update(text).digest("hex");

function merged(options) {
  const Obfuscator = require(PACKAGE);
  return { ...Obfuscator.getOptionsByPreset(options.optionsPreset ?? "default"), ...options };
}

function run(command, args, cwd, timeoutMs, maxBuffer) {
  const started = performance.now();
  const result = spawnSync(command, args, { cwd, encoding: "utf8", timeout: timeoutMs, killSignal: "SIGKILL", maxBuffer });
  return {
    stdout: result.stdout ?? "",
    stderr: (result.stderr ?? "").slice(0, 4000),
    errorLine: (result.stderr ?? "").split("\n").find((line) => /^(\w*Error\b|FATAL ERROR)/.test(line)) ?? "",
    status: result.status,
    timedOut: result.error?.code === "ETIMEDOUT" || (result.signal === "SIGKILL" && result.status === null),
    error: result.error && result.error.code !== "ETIMEDOUT" ? String(result.error.code ?? result.error.message) : undefined,
    ms: performance.now() - started,
  };
}

const HOST_PRELOAD = 'globalThis.document = { domain: "example.com", location: { hostname: "example.com", href: "https://example.com/" } };\n';
const PINNED_CONSOLE = 'for (const key of ["log", "warn", "info", "error", "exception", "table", "trace"]) Object.defineProperty(console, key, { value: console[key] ?? console.log, writable: false, configurable: false });\n';

function runNode(file, cwd, config, seconds = config.timeout, preload = "host.cjs") {
  const result = run("prlimit", ["--core=0", "timeout", "-k", "2", String(seconds), process.execPath, "--no-warnings", `--max-old-space-size=${config.nodeHeapMb}`, "--max-semi-space-size=8", "--require", join(config.out, preload), file], cwd, (seconds + 5) * 1000, 4 * 1024 * 1024);
  if (result.error === "ENOBUFS") return { stdout: `${result.stdout.slice(0, 2000)}\n[floods stdout]`, status: null, timedOut: false, stderr: "output floods stdout past 4 MiB", ms: result.ms };
  return { stdout: result.stdout, status: result.status, timedOut: result.timedOut || result.status === 124 || result.status === 137, stderr: `${result.errorLine}\n${result.stderr.slice(0, 600)}`, ms: result.ms };
}

function obfuscateChain(source, layers, caseId, maxBytes, maxLayerInput) {
  const Obfuscator = require(PACKAGE);
  let code = source;
  for (let layer = 0; layer < layers.length; layer++) {
    if (Buffer.byteLength(code) > maxLayerInput) return { skipped: `input to layer ${layer + 1} is ${Buffer.byteLength(code)} bytes, over ${maxLayerInput}` };
    const seed = parseInt(sha(`ox1/${caseId}/${layer}`).slice(0, 8), 16) % 1_000_000_000;
    code = `${Obfuscator.obfuscate(code, { ...withDictionary(layers[layer].options), seed }).getObfuscatedCode()}\n`;
    if (Buffer.byteLength(code) > maxBytes) return { skipped: `layer ${layer + 1} is ${Buffer.byteLength(code)} bytes, over ${maxBytes}` };
  }
  return { code };
}

function obfuscateInChild(item, config) {
  const input = JSON.stringify({ source: programs()[item.source], layers: item.layers, id: item.id, maxBytes: config.maxBytes, maxLayerInput: config.maxLayerInput });
  const result = spawnSync(process.execPath, [`--max-old-space-size=${config.workerHeapMb}`, fileURLToPath(import.meta.url), "--obfuscate"], { input, encoding: "utf8", maxBuffer: 64 * 1024 * 1024, timeout: config.untwistTimeout * 1000, killSignal: "SIGKILL" });
  if (result.status !== 0) throw new Error(result.error ? String(result.error.code ?? result.error.message) : (result.stderr.split("\n").find((line) => /Error|ERROR/.test(line)) ?? `exit ${result.status}`));
  const made = JSON.parse(result.stdout);
  if (made.error) throw new Error(made.error);
  return made;
}

function obfuscateFromStdin() {
  const job = JSON.parse(readFileSync(0, "utf8"));
  let made;
  try {
    made = obfuscateChain(job.source, job.layers, job.id, job.maxBytes, job.maxLayerInput);
  } catch (error) {
    made = { error: String(error.message).slice(0, 300) };
  }
  process.stdout.write(JSON.stringify(made));
}

function hangs(layers) {
  return layers.some((layer) => {
    const options = merged(layer.options);
    return options.debugProtection === true && options.debugProtectionInterval > 0;
  });
}

function silences(layers) {
  return layers.some((layer) => merged(layer.options).disableConsoleOutput === true);
}

function walk(node, visit, parent) {
  if (!node || typeof node.type !== "string") return;
  visit(node, parent);
  for (const key of Object.keys(node)) {
    const value = node[key];
    if (Array.isArray(value)) value.forEach((child) => child && typeof child.type === "string" && walk(child, visit, node));
    else if (value && typeof value.type === "string") walk(value, visit, node);
  }
}

function selfRedefining(fn) {
  const name = fn.id?.name;
  if (!name) return false;
  let found = false;
  walk(fn.body, (node) => {
    if (node.type === "AssignmentExpression" && node.left.type === "Identifier" && node.left.name === name && /Function/.test(node.right.type)) found = true;
  });
  return found;
}

function inertVar(statement) {
  return statement.type === "VariableDeclaration" && statement.kind === "var" && statement.declarations.every((declaration) => declaration.init === null && declaration.id.type === "Identifier");
}

function fingerprints(code) {
  const acorn = require("acorn");
  const hits = new Set();
  const applyCalls = new Set();
  const nulled = new Set();
  let ast;
  try {
    ast = acorn.parse(code, { ecmaVersion: "latest", sourceType: "script", allowReturnOutsideFunction: true, allowHashBang: true });
  } catch {
    try {
      ast = acorn.parse(code, { ecmaVersion: "latest", sourceType: "module" });
    } catch (error) {
      return { hits: ["unparsable"], error: String(error.message) };
    }
  }
  walk(ast, (node) => {
    if (node.type === "FunctionDeclaration" || node.type === "FunctionExpression") {
      if (selfRedefining(node)) hits.add("self-redefining-function");
      if (selfRedefining(node) && node.body.body.some(inertVar)) hits.add("inert-var-in-machinery");
    }
    if (node.type === "ArrayExpression" && node.elements.length >= 4 && node.elements.every((element) => element?.type === "Literal" && typeof element.value === "string") && node.elements.some((element) => /^[\w+/=]{4,}$/.test(element.value) && /[A-Z]/.test(element.value) && /[a-z]/.test(element.value) && /\d/.test(element.value))) hits.add("encoded-string-array");
    if (node.type === "CallExpression" && node.callee.type === "MemberExpression" && memberName(node.callee) === "push" && node.arguments[0]?.type === "CallExpression" && node.arguments[0].callee.type === "MemberExpression" && memberName(node.arguments[0].callee) === "shift") hits.add("rotator");
    if (node.type === "DebuggerStatement") hits.add("debug-protection");
    if (node.type === "Literal" && typeof node.value === "string") literalHits(node.value, hits);
    if (node.type === "TemplateLiteral") node.quasis.forEach((quasi) => literalHits(quasi.value.cooked ?? "", hits));
    if (node.type === "Literal" && node.regex?.pattern === "(((.+)+)+)+$") hits.add("self-defending");
    if (node.type === "CallExpression" && node.callee.type === "MemberExpression" && memberName(node.callee) === "replace" && node.callee.object.type === "Literal" && typeof node.callee.object.value === "string" && node.arguments[1]?.type === "Literal" && node.arguments[1].value === "") hits.add("domain-lock");
    if (node.type === "CallExpression" && node.callee.type === "MemberExpression" && memberName(node.callee) === "apply" && node.arguments[1]?.type === "Identifier" && node.arguments[1].name === "arguments") applyCalls.add(node);
    if (node.type === "AssignmentExpression" && node.right.type === "Literal" && node.right.value === null && node.left.type === "Identifier") nulled.add(node.left.name);
  });
  for (const call of applyCalls) if (call.callee.object.type === "Identifier" && nulled.has(call.callee.object.name)) hits.add("call-controller");
  if (/\bfromCharCode\b[\s\S]{0,400}%\s*0x100|%\s*256/.test(code) && /charCodeAt/.test(code)) hits.add("rc4-decoder");
  return { hits: [...hits].sort() };
}

function literalHits(value, hits) {
  if (value.includes("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=")) hits.add("base64-alphabet");
  if (value.includes("(((.+)+)+)+$")) hits.add("self-defending");
  if (/^(debu|gger|stateObject)$|debugger|while \(true\) \{\}/.test(value)) hits.add("debug-protection");
  if (/^(exception|table|trace)$/.test(value)) hits.add("console-disabling");
  if (/example\.(com|org)|about:blank/.test(value)) hits.add("domain-lock");
  if (value.includes('{}.constructor("return this")( )')) hits.add("global-getter");
}

function memberName(member) {
  if (!member.computed && member.property.type === "Identifier") return member.property.name;
  if (member.property.type === "Literal") return String(member.property.value);
  return null;
}

function machineryCounts(report) {
  const counts = {};
  for (const stage of report?.stages ?? []) {
    for (const [pass, count] of [...(stage.per_pass ?? []), ...(stage.fixpoint_per_pass ?? [])]) {
      if (MACHINERY_PASSES.includes(pass) && count > 0) counts[pass] = (counts[pass] ?? 0) + count;
    }
  }
  return counts;
}

function acquire(gate, limit) {
  for (;;) {
    const held = Atomics.load(gate, 0);
    if (held < limit && Atomics.compareExchange(gate, 0, held, held + 1) === held) return;
    Atomics.wait(gate, 0, held, 200);
  }
}

function release(gate) {
  Atomics.sub(gate, 0, 1);
  Atomics.notify(gate, 0);
}

function untwist(file, lane, dir, config) {
  const args = ["--report", "json", "--max-memory", config.untwistMemory, ...(lane === "unsafe" ? ["--unsafe"] : []), file];
  const big = statSync(file).size > config.bigInput;
  if (big) acquire(workerData.gate, config.bigJobs);
  let result;
  try {
    result = run(config.untwist, args, dir, config.untwistTimeout * 1000, 64 * 1024 * 1024);
  } finally {
    if (big) release(workerData.gate);
  }
  let report = null;
  try {
    report = JSON.parse(result.stdout);
  } catch {
    report = null;
  }
  return { result, report };
}

function laneRun(lane, obfFile, dir, config, expect) {
  const { result, report } = untwist(obfFile, lane, dir, config);
  const base = { lane, untwistMs: result.ms, status: result.status };
  if (result.timedOut) return { ...base, verdict: "fail", reason: "untwist-timeout" };
  if (!report || typeof report.code !== "string") return { ...base, verdict: "fail", reason: result.status === null ? "untwist-killed" : "untwist-error", stderr: result.stderr.slice(0, 800) };
  const outFile = join(dir, `${lane}.js`);
  writeFileSync(outFile, report.code.endsWith("\n") ? report.code : `${report.code}\n`);
  const notes = report.notes ?? [];
  const printed = runNode(outFile, dir, config, expect.hangs ? config.hangTimeout : config.timeout);
  const second = untwist(outFile, "unsafe", dir, config);
  const residue = { ...fingerprints(report.code), rerun: machineryCounts(second.report) };
  const leftover = residue.hits.length > 0 || Object.keys(residue.rerun).length > 0;
  const matches = (stdout) => stdout === expect.source.stdout;
  const panicked = (report.panicked ?? []).length > 0 || report.unpack_panicked === true;
  const budget = (report.budget?.hits ?? []).map((hit) => hit.kind ?? hit.hit ?? JSON.stringify(hit));
  const out = { ...base, budget, nodeMs: printed.ms, outBytes: Buffer.byteLength(report.code), notes, residue, leftover, panicked, stderr: printed.stderr, printed: printed.stdout.slice(0, 2000), timedOut: printed.timedOut, exit: printed.status };
  const sameAsSource = matches(printed.stdout) && printed.status === expect.source.status && !printed.timedOut;
  const hangsLikeObf = expect.hangs && printed.timedOut && matches(printed.stdout);
  const sameAsObf = printed.stdout === expect.obf.stdout && (expect.hangs ? printed.timedOut : printed.status === expect.obf.status && !printed.timedOut);
  return judge({ ...out, sameAsSource, sameAsObf: sameAsObf || hangsLikeObf });
}

function judge(out) {
  const refusals = out.notes.filter((note) => PREMISE_NOTE.test(note));
  const fail = (reason) => ({ ...out, refusals, verdict: "fail", reason });
  if (out.lane === "unsafe") {
    if (!out.sameAsSource) return fail(out.timedOut ? "output-hangs" : "output-differs");
    if (out.leftover) return fail("machinery-left");
    return { ...out, refusals, verdict: "pass", reason: undefined };
  }
  if (!out.sameAsSource && !out.sameAsObf) return fail(out.timedOut ? "output-hangs" : "output-differs");
  if (!out.leftover) return { ...out, refusals, verdict: "pass", reason: undefined };
  if (refusals.length > 0) return { ...out, refusals, verdict: "pass-kept", reason: undefined };
  return fail("machinery-left-without-premise");
}

function sourceExpectation(source, config) {
  const dir = join(config.out, "sources");
  mkdirSync(dir, { recursive: true });
  const file = join(dir, `${source}.js`);
  writeFileSync(file, programs()[source]);
  return runNode(file, dir, config);
}

function runCase(item, config, expected) {
  const timings = {};
  const dir = join(config.out, "cases", item.id);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const started = performance.now();
  let made;
  try {
    made = obfuscateInChild(item, config);
  } catch (error) {
    return finish({ ...item, verdict: "skip", reason: /heap/.test(error.message) ? "obfuscator-out-of-memory" : "obfuscator-threw", detail: String(error.message).slice(0, 300) });
  }
  timings.generate = performance.now() - started;
  if (made.skipped) return finish({ ...item, verdict: "skip", reason: "too-big", detail: made.skipped });
  const obfFile = join(dir, "obf.js");
  writeFileSync(obfFile, made.code);
  item = { ...item, obfSha: sha(made.code).slice(0, 16) };
  const source = expected[item.source];
  const hang = hangs(item.layers);
  const obf = runNode(obfFile, dir, config, hang ? config.hangTimeout : config.timeout);
  timings.nodeObf = obf.ms;
  const settled = hang ? obf.timedOut : !obf.timedOut;
  const same = obf.stdout === source.stdout && (hang || obf.status === source.status) && settled;
  const pinned = !same && silences(item.layers) ? runNode(obfFile, dir, config, hang ? config.hangTimeout : config.timeout, "host-pinned.cjs") : null;
  const silenced = pinned !== null && pinned.stdout === source.stdout && (hang ? pinned.timedOut : !pinned.timedOut && pinned.status === source.status);
  if (!same && !silenced) {
    return finish({ ...item, verdict: "skip", reason: "obfuscated-differs", detail: `exit ${(pinned ?? obf).status}${(pinned ?? obf).timedOut ? " timeout" : ""}: ${(pinned ?? obf).stderr.split("\n")[0] || (pinned ?? obf).stdout.slice(0, 120)}`, bytes: made.code.length });
  }
  const expect = { source, obf, hangs: hang };
  const lanes = {};
  for (const lane of ["default", "unsafe"]) {
    lanes[lane] = laneRun(lane, obfFile, dir, config, expect);
    timings[`untwist-${lane}`] = lanes[lane].untwistMs;
    timings[`node-${lane}`] = lanes[lane].nodeMs;
  }
  return finish({ ...item, verdict: "ran", bytes: made.code.length, obfRun: `${obf.stdout === source.stdout ? "same" : "silenced"}${hang ? "+hangs" : ""}`, lanes, timings });

  function finish(row) {
    row.ms = performance.now() - started;
    const keep = config.keep || (row.lanes && Object.values(row.lanes).some((lane) => lane.verdict === "fail"));
    if (!keep) rmSync(dir, { recursive: true, force: true });
    else writeFileSync(join(dir, "case.json"), `${JSON.stringify({ ...row, timings }, null, 2)}\n`);
    return row;
  }
}

function parseArgs(argv) {
  const config = {
    out: join(homedir(), ".cache/untwist-tmp/ox1/run"),
    untwist: process.env.UNTWIST_BIN ?? join(process.env.CARGO_TARGET_DIR ?? join(REPO, "target"), "release/untwist"),
    jobs: Math.max(1, Math.min(10, cpus().length - 2)),
    random: 32,
    seed: 51,
    timeout: 30,
    hangTimeout: 8,
    nodeHeapMb: 96,
    untwistMemory: "768MiB",
    untwistTimeout: 120,
    maxBytes: 600_000,
    maxLayerInput: 100_000,
    workerHeapMb: 512,
    bigInput: 150_000,
    bigJobs: 2,
    layerStride: 1,
    limit: Infinity,
    filter: null,
    keep: false,
    planOnly: false,
    rejudge: null,
  };
  for (let i = 0; i < argv.length; i++) {
    const flag = argv[i];
    const value = () => argv[++i];
    if (flag === "--out") config.out = resolve(value());
    else if (flag === "--untwist") config.untwist = resolve(value());
    else if (flag === "--jobs") config.jobs = Number(value());
    else if (flag === "--random") config.random = Number(value());
    else if (flag === "--seed") config.seed = Number(value());
    else if (flag === "--timeout") config.timeout = Number(value());
    else if (flag === "--hang-timeout") config.hangTimeout = Number(value());
    else if (flag === "--untwist-timeout") config.untwistTimeout = Number(value());
    else if (flag === "--max-bytes") config.maxBytes = Number(value());
    else if (flag === "--big-jobs") config.bigJobs = Number(value());
    else if (flag === "--max-layer-input") config.maxLayerInput = Number(value());
    else if (flag === "--layer-stride") config.layerStride = Number(value());
    else if (flag === "--limit") config.limit = Number(value());
    else if (flag === "--filter") config.filter = new RegExp(value());
    else if (flag === "--keep") config.keep = true;
    else if (flag === "--plan") config.planOnly = true;
    else if (flag === "--rejudge") config.rejudge = resolve(value());
    else throw new Error(`unknown flag ${flag}`);
  }
  return config;
}

function pct(part, whole) {
  return whole === 0 ? "-" : `${((100 * part) / whole).toFixed(1)}%`;
}

function summarise(rows) {
  const ran = rows.filter((row) => row.verdict === "ran");
  const groups = { all: ran, ...Object.fromEntries([1, 2, 3].map((depth) => [`layers=${depth}`, ran.filter((row) => row.depth === depth)])), same: ran.filter((row) => !row.mixed), mixed: ran.filter((row) => row.mixed) };
  const table = {};
  for (const [name, list] of Object.entries(groups)) {
    table[name] = {};
    for (const lane of ["default", "unsafe"]) {
      const pass = list.filter((row) => row.lanes[lane].verdict === "pass").length;
      const kept = list.filter((row) => row.lanes[lane].verdict === "pass-kept").length;
      table[name][lane] = { cases: list.length, pass, kept, fail: list.length - pass - kept, rate: pct(pass + kept, list.length) };
    }
  }
  const skips = {};
  for (const row of rows.filter((row) => row.verdict === "skip")) skips[row.reason] = (skips[row.reason] ?? 0) + 1;
  return { total: rows.length, ran: ran.length, skips, table };
}

function rejudge(dir) {
  const rows = readFileSync(join(dir, "results.jsonl"), "utf8").trim().split("\n").map((line) => JSON.parse(line));
  const judged = rows.map((row) => (row.verdict === "ran" ? { ...row, lanes: Object.fromEntries(Object.entries(row.lanes).map(([lane, out]) => [lane, "sameAsSource" in out ? judge(out) : out])) } : row));
  writeFileSync(join(dir, "results.jsonl"), judged.map((row) => JSON.stringify(row)).join("\n") + "\n");
  const previous = JSON.parse(readFileSync(join(dir, "summary.json"), "utf8"));
  const summary = { ...previous, ...summarise(judged) };
  writeFileSync(join(dir, "summary.json"), `${JSON.stringify(summary, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify(summary, null, 2)}\n`);
}

async function main() {
  const config = parseArgs(process.argv.slice(2));
  if (config.rejudge) return rejudge(config.rejudge);
  const planned = plan(config);
  let cases = planned.cases.filter((item) => !config.filter || config.filter.test(item.id));
  cases = cases.slice(0, config.limit);
  const domainSizes = Object.fromEntries(Object.entries(planned.domains).map(([name, values]) => [name, values.length]));
  process.stderr.write(`${Object.keys(planned.domains).length} options ${JSON.stringify(domainSizes)}\n${planned.pairs} valid value pairs (${planned.invalidPairs} invalid), ${planned.pairwiseRows} pairwise rows, ${planned.combos.length} combos, ${cases.length} cases\n`);
  if (config.planOnly) return;
  if (!existsSync(config.untwist)) throw new Error(`no untwist binary at ${config.untwist}`);
  mkdirSync(config.out, { recursive: true });
  writeFileSync(join(config.out, "host.cjs"), HOST_PRELOAD);
  writeFileSync(join(config.out, "host-pinned.cjs"), HOST_PRELOAD + PINNED_CONSOLE);
  for (const source of Object.keys(programs())) {
    const hits = fingerprints(programs()[source]).hits;
    if (hits.length > 0) throw new Error(`fingerprints fire on source ${source}: ${hits}`);
  }
  const expected = Object.fromEntries(Object.keys(programs()).map((source) => [source, sourceExpectation(source, config)]));
  writeFileSync(join(config.out, "plan.json"), `${JSON.stringify({ config: { ...config, filter: String(config.filter) }, domains: planned.domains, combos: planned.combos }, null, 2)}\n`);
  const results = join(config.out, "results.jsonl");
  writeFileSync(results, "");
  const started = performance.now();
  const rows = [];
  let next = 0;
  const gate = new Int32Array(new SharedArrayBuffer(4));
  const record = (row) => {
    rows.push(row);
    appendFileSync(results, `${JSON.stringify(row)}\n`);
    if (rows.length % 50 === 0 || rows.length === cases.length) process.stderr.write(`${rows.length}/${cases.length} in ${((performance.now() - started) / 1000).toFixed(0)} s\n`);
  };
  const lane = () => new Promise((done) => {
    const worker = new Worker(fileURLToPath(import.meta.url), { workerData: { config: { ...config, filter: null }, expected, gate } });
    let current = null;
    const feed = () => {
      if (next >= cases.length) {
        current = null;
        worker.terminate().then(() => done());
        return;
      }
      current = cases[next++];
      worker.postMessage(current);
    };
    worker.on("message", (row) => {
      record(row);
      feed();
    });
    worker.on("error", (error) => {
      if (current) record({ ...current, verdict: "skip", reason: "harness-error", detail: String(error.message).slice(0, 200) });
      current = null;
      lane().then(done);
    });
    feed();
  });
  await Promise.all(Array.from({ length: Math.min(config.jobs, cases.length) }, lane));
  const summary = { ...summarise(rows), seconds: Math.round((performance.now() - started) / 1000), jobs: config.jobs, untwist: config.untwist, node: process.version };
  writeFileSync(join(config.out, "summary.json"), `${JSON.stringify(summary, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify(summary, null, 2)}\n`);
}

if (isMainThread && process.argv[2] === "--obfuscate") {
  obfuscateFromStdin();
} else if (isMainThread) {
  await main();
} else {
  parentPort.on("message", (item) => {
    let row;
    try {
      row = runCase(item, workerData.config, workerData.expected);
    } catch (error) {
      row = { ...item, verdict: "skip", reason: "harness-error", detail: String(error.stack).slice(0, 500) };
    }
    parentPort.postMessage(row);
  });
}
