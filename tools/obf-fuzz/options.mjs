const DICTIONARY = ["amber", "birch", "cedar", "dune", "ember", "fjord", "grove", "heath", "inlet", "jade", "kelp", "larch", "moss", "nook", "opal", "pine", "quill", "reed"];

export const OBFUSCATORS = { "4.2.2": "javascript-obfuscator", "5.9.1": "javascript-obfuscator-5" };

const GROUPS = [
  (r) => ({ compact: r.chance(0.5) }),
  (r) => ({ simplify: r.chance(0.5) }),
  (r) => ({ target: r.pick(["browser", "browser-no-eval", "node", "service-worker"]) }),
  (r) => ({ controlFlowFlattening: true, controlFlowFlatteningThreshold: r.pick([0.25, 0.75, 1]) }),
  (r) => ({ deadCodeInjection: true, deadCodeInjectionThreshold: r.pick([0.2, 0.4, 1]) }),
  () => ({ disableConsoleOutput: true }),
  (r) => ({ identifierNamesGenerator: r.pick(["hexadecimal", "mangled", "mangled-shuffled"]) }),
  () => ({ identifierNamesGenerator: "dictionary", identifiersDictionary: DICTIONARY }),
  (r) => ({ identifiersPrefix: r.pick(["obf", "q"]) }),
  () => ({ renameGlobals: true }),
  () => ({ numbersToExpressions: true }),
  (r) => ({ splitStrings: true, splitStringsChunkLength: r.pick([2, 3, 5, 10]) }),
  () => ({ transformObjectKeys: true }),
  () => ({ unicodeEscapeSequence: true }),
  () => ({ stringArray: false }),
  (r) => ({ stringArrayThreshold: r.pick([0, 0.5, 0.75, 1]) }),
  (r) => ({ stringArrayRotate: r.chance(0.5) }),
  (r) => ({ stringArrayShuffle: r.chance(0.5) }),
  (r) => ({ stringArrayIndexShift: r.chance(0.5) }),
  (r) => ({ stringArrayEncoding: r.pick([["base64"], ["rc4"], ["none", "base64", "rc4"], ["base64", "rc4"]]) }),
  (r) => ({ stringArrayIndexesType: r.pick([["hexadecimal-numeric-string"], ["hexadecimal-number", "hexadecimal-numeric-string"]]) }),
  (r) => ({ stringArrayCallsTransform: true, stringArrayCallsTransformThreshold: r.pick([0.5, 1]) }),
  (r) => ({ stringArrayWrappersCount: r.pick([0, 1, 2, 5]), stringArrayWrappersChainedCalls: r.chance(0.7) }),
  (r) => ({ stringArrayWrappersType: r.pick(["variable", "function"]), stringArrayWrappersParametersMaxCount: r.pick([2, 3, 5]) }),
  () => ({ stringArrayThreshold: 0, forceTransformStrings: ["^[a-m]", "s$"] }),
  () => ({ reservedNames: ["^show_", "^tag_"] }),
  () => ({ stringArrayThreshold: 1, reservedStrings: ["^[n-z]"] }),
  () => ({ ignoreImports: true }),
];

const RESIDUE_ONLY_GROUPS = [
  (r) => ({ debugProtection: true, ...(r.chance(0.5) ? { debugProtectionInterval: 4000 } : {}) }),
  () => ({ selfDefending: true }),
  (r) => (r.chance(0.5) ? { domainLock: ["example.com"] } : { domainLock: [".example.com", "example.org"], domainLockRedirectUrl: "https://example.com/locked" }),
  (r) => ({ renameProperties: true, renamePropertiesMode: r.pick(["safe", "unsafe"]) }),
];
const RESIDUE_ONLY_RATE = 0.12;

const PRESETS = ["default", "low-obfuscation", "medium-obfuscation", "high-obfuscation"];
const HOSTILE = ["debugProtection", "selfDefending", "domainLock"];
const MAX_GROUPS = 5;

export function sampleOptions(r) {
  const options = r.chance(0.12) ? { optionsPreset: r.pick(PRESETS) } : {};
  const count = r.range(1, MAX_GROUPS);
  for (const group of r.sample(GROUPS, count)) Object.assign(options, group(r));
  if (r.chance(RESIDUE_ONLY_RATE)) Object.assign(options, r.pick(RESIDUE_ONLY_GROUPS)(r));
  return {
    obfuscator: r.chance(0.75) ? "5.9.1" : "4.2.2",
    options,
    seed: r.int(1_000_000_000),
  };
}

export function behaviourOf(options, presetOptions) {
  const merged = { ...presetOptions, ...options };
  const set = (key) => (Array.isArray(merged[key]) ? merged[key].length > 0 : merged[key] === true);
  if (HOSTILE.some(set)) return "residue-only:hostile";
  if (merged.renameProperties === true) return "residue-only:renamed-properties";
  return "node";
}
