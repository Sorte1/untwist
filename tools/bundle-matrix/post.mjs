import { require, tool } from "./lib.mjs";

const SEED = 5101;
const DICTIONARY = ["i", "ì", "í", "î", "ï"].flatMap((a) => ["i", "ì", "í", "î", "ï"].map((b) => `i${a}${b}`));

const obfuscate = (alias, options) => (code) =>
  require(alias).obfuscate(code, { seed: SEED, target: "node", ...options }).getObfuscatedCode();

const STRINGS = { stringArray: true, stringArrayThreshold: 1 };

const OBFUSCATIONS = {
  "obf-default": {},
  "obf-strings-base64": { ...STRINGS, stringArrayEncoding: ["base64"], stringArrayRotate: true, stringArrayShuffle: true },
  "obf-strings-rc4-wrappers": {
    ...STRINGS, stringArrayEncoding: ["rc4"], stringArrayWrappersCount: 2, stringArrayWrappersType: "function",
    stringArrayWrappersChainedCalls: true, stringArrayWrappersParametersMaxCount: 4, stringArrayIndexShift: true,
  },
  "obf-strings-none": { ...STRINGS, stringArrayEncoding: ["none"], stringArrayIndexesType: ["hexadecimal-number", "hexadecimal-numeric-string"] },
  "obf-cff": { controlFlowFlattening: true, controlFlowFlatteningThreshold: 0.75 },
  "obf-dead-code": { deadCodeInjection: true, deadCodeInjectionThreshold: 0.4 },
  "obf-self-defending": { selfDefending: true, compact: true },
  "obf-object-keys": { transformObjectKeys: true },
  "obf-unicode": { unicodeEscapeSequence: true },
  "obf-split-strings": { splitStrings: true, splitStringsChunkLength: 5 },
  "obf-numbers": { numbersToExpressions: true, simplify: true },
  "obf-rename-globals": { renameGlobals: true },
  "obf-rename-properties": { renameProperties: true, renamePropertiesMode: "safe" },
  "obf-dictionary": {
    ...STRINGS, stringArrayEncoding: ["none"], stringArrayRotate: true, selfDefending: true, compact: true,
    identifierNamesGenerator: "dictionary", identifiersDictionary: DICTIONARY.flatMap((a) => DICTIONARY.map((b) => a + b.slice(1))),
  },
  "obf-heavy": {
    ...STRINGS, stringArrayEncoding: ["base64", "rc4"], stringArrayWrappersCount: 1, stringArrayRotate: true, stringArrayShuffle: true,
    controlFlowFlattening: true, controlFlowFlatteningThreshold: 0.5, deadCodeInjection: true, deadCodeInjectionThreshold: 0.2,
    selfDefending: true, transformObjectKeys: true, identifierNamesGenerator: "mangled-shuffled",
  },
};

function terser(code) {
  return require("terser").minify(code, { compress: { passes: 2 }, mangle: true, format: { comments: false } }).then((r) => r.code);
}

async function esbuildMinify(code) {
  return (await require("esbuild").transform(code, { minify: true, loader: "js" })).code;
}

async function swcMinify(code) {
  return (await require("@swc/core").minify(code, { compress: true, mangle: true })).code;
}

function uglify(code) {
  const result = require("uglify-js").minify(code, { compress: { passes: 2 }, mangle: true });
  if (result.error) throw result.error;
  return result.code;
}

export const MINIFIERS = {
  terser: { tool: () => tool("terser"), flags: "compress.passes=2 mangle comments=false", run: terser },
  "esbuild-min": { tool: () => tool("esbuild"), flags: "transform --minify", run: esbuildMinify },
  "swc-min": { tool: () => tool("@swc/core"), flags: "minify compress mangle", run: swcMinify },
  uglify: { tool: () => tool("uglify-js"), flags: "compress.passes=2 mangle", run: uglify },
};

export const OBFUSCATORS = Object.fromEntries([
  ...Object.entries(OBFUSCATIONS).map(([name, options]) => [name, {
    tool: () => tool("javascript-obfuscator"),
    flags: `seed=${SEED} target=node ${JSON.stringify(options).replace(/"identifiersDictionary":\[[^\]]*\]/, '"identifiersDictionary":[i+iìíîï x4]')}`,
    run: obfuscate("javascript-obfuscator", options),
  }]),
  ["obf4-default", { tool: () => tool("javascript-obfuscator", "javascript-obfuscator-4"), flags: `seed=${SEED} target=node {}`, run: obfuscate("javascript-obfuscator-4", {}) }],
]);
