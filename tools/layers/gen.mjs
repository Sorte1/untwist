import { createHash } from "node:crypto";
import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";
import { PACKER_SOURCE, packBase62, packCustomAlphabet, packerVersion, withoutFastPath } from "./pack.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../..");
const OUT = join(ROOT, "fixtures/layers");
const SRC = join(HERE, "src");

const b64 = (text) => Buffer.from(text, "utf8").toString("base64");
const viaAtob = (text) => `eval(atob(${JSON.stringify(b64(text))}));`;

function nestAtob(depth, inner) {
  let text = inner;
  for (let i = 0; i < depth; i += 1) text = viaAtob(text);
  return text;
}

function nestInline(generations, inner) {
  let text = inner;
  for (let i = 1; i < generations; i += 1) text = `eval(${JSON.stringify(text)})`;
  return text;
}

function quine() {
  const template = "eval(Q.split(String.fromCharCode(81)).join(JSON.stringify(Q)));";
  return template.split("Q").join(JSON.stringify(template));
}

function pingPong() {
  const toggle =
    ".split(String.fromCharCode(37,49,37)).join(String.fromCharCode(37,48,37))" +
    ".split(String.fromCharCode(37,50,37)).join(String.fromCharCode(37,49,37))" +
    ".split(String.fromCharCode(37,48,37)).join(String.fromCharCode(37,50,37))";
  const template = `eval(Q.split(String.fromCharCode(81)).join(JSON.stringify(Q))${toggle});/*%1%*/`;
  return template.split("Q").join(JSON.stringify(template));
}

const brackets = (n) => "[".repeat(n) + "]".repeat(n);

function generatedEval() {
  const say = (label) => `console.log(${JSON.stringify(label)}, [1, 2, 3].join("-"), typeof globalThis.layerProbe);`;
  const codes = (text) => [...text].map((ch) => ch.charCodeAt(0)).join(", ");
  const escaped = (text) => [...text].map((ch) => "%" + ch.charCodeAt(0).toString(16).padStart(2, "0")).join("");
  return {
    "eval/atob.js": viaAtob(say("atob")),
    "eval/fromcharcode.js": `eval(String.fromCharCode(${codes(say("fromCharCode"))}));`,
    "eval/unescape.js": `eval(unescape(${JSON.stringify(escaped(say("unescape")))}));`,
    "eval/nested-2.js": nestAtob(2, say("nested 2")),
    "eval/nested-3.js": `eval(${JSON.stringify(nestAtob(2, say("nested 3")))});`,
    "eval/nested-17.js": nestAtob(17, say("nested 17")),
    "eval/cycle-quine.js": quine(),
    "eval/cycle-two.js": pingPong(),
    "eval/limit-k3-deep.js": `eval(${JSON.stringify(`console.log("k3 deep", ${brackets(300)}.length)`)});`,
    "inline/refuse-gen5.js": `function site() {\n  ${nestInline(5, "console.log('generation 5', typeof site)")};\n}\nsite();\n`,
    "inline/refuse-deep.js": `console.log("k4 deep", eval(${JSON.stringify(brackets(600))}).length);\n`,
  };
}

const source = (name) => readFileSync(join(SRC, name), "utf8");

function radixOf(packed) {
  const match = /\}\('(?:[^'\\]|\\.)*',(\d+),(\d+),/s.exec(packed);
  if (!match) throw new Error("no packer arguments");
  return Number(match[1]);
}

function expectRadix(name, packed, low, high) {
  const a = radixOf(packed);
  if (a < low || a > high) throw new Error(`${name}: radix ${a} outside ${low}..${high}`);
  return packed;
}

function generatedPacker() {
  const webpack = readFileSync(join(ROOT, "fixtures/unpack-bundlers/webpack/min-webpack5-locals.js"), "utf8");
  const medium = packBase62(source("medium.js"));
  return {
    "packer/r10.js": expectRadix("r10", packBase62(source("small.js")), 2, 10),
    "packer/r36.js": expectRadix("r36", medium, 11, 36),
    "packer/r62.js": expectRadix("r62", packBase62(source("large.js")), 37, 62),
    "packer/custom-alphabet.js": packCustomAlphabet(source("medium.js")),
    "packer/no-fast-path.js": withoutFastPath(medium),
    "packer/empty-slots.js": packBase62(source("digits.js")),
    "packer/webpack.js": packBase62(webpack),
    "packer/nested.js": packBase62(packBase62(source("small.js"))),
  };
}

function checkComputed(name, text) {
  if (name.startsWith("eval/cycle-")) {
    const inner = /^eval\((.*)\);(\/\*%[12]%\*\/)?$/s.exec(text)[1];
    const once = vm.runInNewContext(inner);
    const twice = vm.runInNewContext(/^eval\((.*)\);(\/\*%[12]%\*\/)?$/s.exec(once)[1]);
    const period = once === text ? 1 : twice === text ? 2 : 0;
    if (period === 0) throw new Error(`${name} is not a cycle`);
    return `cycle period ${period}`;
  }
  return "";
}

const FORMS = {
  "eval/literal.js": ["W-eval", "peel"],
  "eval/join.js": ["W-eval", "peel"],
  "eval/reverse.js": ["W-eval", "peel"],
  "eval/atob.js": ["W-eval", "peel"],
  "eval/fromcharcode.js": ["W-eval", "peel"],
  "eval/unescape.js": ["W-eval", "peel"],
  "eval/indirect-comma.js": ["W-eval", "peel"],
  "eval/indirect-globalthis.js": ["W-eval", "peel"],
  "eval/indirect-this.js": ["W-eval", "peel"],
  "eval/nested-2.js": ["W-eval", "peel*2"],
  "eval/nested-3.js": ["W-eval", "peel*3"],
  "eval/nested-17.js": ["W-eval", "peel*16,refuse:LAYER_CAP"],
  "eval/cycle-quine.js": ["W-eval", "refuse:LAYER_CYCLE"],
  "eval/cycle-two.js": ["W-eval", "peel,refuse:LAYER_CYCLE"],
  "eval/decl-var.js": ["W-eval", "peel"],
  "eval/decl-let.js": ["W-eval", "needs-unsafe"],
  "eval/decl-delete.js": ["W-eval", "needs-unsafe:a-l1-witness"],
  "eval/decl-delete-ident.js": ["W-eval", "needs-unsafe:a-l1-witness"],
  "eval/decl-descriptor.js": ["W-eval", "needs-unsafe:a-l1-witness"],
  "eval/decl-with.js": ["W-eval", "needs-unsafe:a-l1-witness"],
  "eval/strict-payload.js": ["W-eval", "needs-unsafe:a-l1-witness"],
  "eval/strict-nodecl.js": ["W-eval", "peel"],
  "eval/neg-not-string.js": ["W-eval", "refuse:LAYER_NOT_A_STRING"],
  "eval/neg-host-read.js": ["W-eval", "refuse:LAYER_BAILED"],
  "eval/neg-bail.js": ["W-eval", "refuse:LAYER_BAILED"],
  "eval/neg-unparsable.js": ["W-eval", "refuse:LAYER_UNPARSABLE"],
  "eval/limit-k3-deep.js": ["W-eval", "refuse:LAYER_TOO_DEEP"],
  "eval/limit-k8-fuel.js": ["W-eval", "refuse:LAYER_FUEL"],
  "function/call.js": ["W-Function", "peel"],
  "function/new-args.js": ["W-Function", "peel"],
  "function/injection.js": ["W-Function", "refuse:LAYER_INJECTION"],
  "function/this-strict.js": ["W-Function", "peel"],
  "function/joined-params.js": ["W-Function", "peel"],
  "packer/r10.js": ["W-packer", "peel"],
  "packer/r36.js": ["W-packer", "peel"],
  "packer/r62.js": ["W-packer", "peel"],
  "packer/custom-alphabet.js": ["W-packer", "peel"],
  "packer/no-fast-path.js": ["W-packer", "peel"],
  "packer/empty-slots.js": ["W-packer", "peel"],
  "packer/webpack.js": ["W-packer", "peel,unpack"],
  "packer/nested.js": ["W-packer", "peel*2"],
  "inline/stmt.js": ["I-stmt", "peel"],
  "inline/expr.js": ["I-expr", "peel"],
  "inline/function.js": ["I-Function", "peel"],
  "inline/function-new.js": ["I-Function", "peel"],
  "inline/refuse-captured.js": ["I-Function", "refuse:LAYER_SHAPE"],
  "inline/strict-payload.js": ["I-stmt", "peel"],
  "inline/refuse-await.js": ["I-expr", "refuse:LAYER_SHAPE"],
  "inline/refuse-decl.js": ["I-stmt", "refuse:LAYER_SHAPE"],
  "inline/refuse-indirect.js": ["I-eval-indirect", "refuse:LAYER_SHAPE"],
  "inline/refuse-gen5.js": ["I-stmt", "refuse:LAYER_SHAPE;unsafe=peel*4"],
  "inline/refuse-deep.js": ["I-expr", "refuse:LAYER_TOO_DEEP"],
};

function listFixtures(dir) {
  return readdirSync(dir)
    .flatMap((name) => {
      const path = join(dir, name);
      if (statSync(path).isDirectory()) return listFixtures(path);
      return name.endsWith(".js") ? [path] : [];
    })
    .sort();
}

const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

function origin(rel, generated) {
  if (!generated.has(rel)) return "hand";
  return rel.startsWith("packer/") ? "pack.mjs" : "gen.mjs";
}

function writeManifest(generated) {
  const lines = [
    `# node ${process.version}; ${packerVersion()}: tarball sha1 db5a5a81a37f94afc26e0b76b86fd2155006c0d3,`,
    `# integrity sha512-W1jRZDooIanD8B2mRNsVYH2n+4tlJMybUsb0GSmK6r1I3j2TzwaknRE3QwQpFHy8lgi4X1i/cXYiDNMbGDIVQQ==,`,
    `# vendored packer.js sha256 ${sha256(readFileSync(PACKER_SOURCE))}`,
    "# path\tform\torigin\texpect\tsha256",
  ];
  for (const path of listFixtures(OUT)) {
    const rel = relative(OUT, path);
    const entry = FORMS[rel];
    if (!entry) throw new Error(`${rel} has no form in gen.mjs`);
    lines.push([rel, entry[0], origin(rel, generated), entry[1], sha256(readFileSync(path))].join("\t"));
  }
  const missing = Object.keys(FORMS).filter((rel) => !lines.some((line) => line.startsWith(rel + "\t")));
  if (missing.length) throw new Error(`fixtures missing: ${missing.join(", ")}`);
  writeFileSync(join(OUT, "MANIFEST"), lines.join("\n") + "\n");
  return lines.length - 4;
}

const generated = { ...generatedEval(), ...generatedPacker() };
for (const [rel, text] of Object.entries(generated)) {
  const body = text.endsWith("\n") ? text : text + "\n";
  const note = checkComputed(rel, text);
  writeFileSync(join(OUT, rel), body);
  if (note) console.log(`${rel}: ${note}`);
}
console.log(`wrote ${Object.keys(generated).length} generated fixtures; MANIFEST lists ${writeManifest(new Set(Object.keys(generated)))}`);
