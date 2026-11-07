export const NA = "-";

export const VERSIONS = [
  "javascript-obfuscator-0.10.2",
  "javascript-obfuscator-0.16.0",
  "javascript-obfuscator-0.18.8",
  "javascript-obfuscator-0.20.4",
  "javascript-obfuscator-0.23.2",
  "javascript-obfuscator-0.28.5",
  "javascript-obfuscator-1.4.0",
  "javascript-obfuscator-1.6.0",
  "javascript-obfuscator-1.12.1",
  "javascript-obfuscator-2.4.3",
  "javascript-obfuscator-2.9.6",
  "javascript-obfuscator-2.11.1",
  "javascript-obfuscator-2.13.0",
  "javascript-obfuscator-2.14.0",
  "javascript-obfuscator-2.15.6",
  "javascript-obfuscator-2.18.1",
  "javascript-obfuscator-2.19.1",
  "javascript-obfuscator-3.2.7",
  "javascript-obfuscator",
  "javascript-obfuscator-5",
];

export const CONTAINERS = ["top", "iife", "wp4-module", "wp4-bundle"];
export const MINIFIERS = ["none", "uglify", "terser"];

const EXTRAS = [
  { name: "none", since: "0.0.0", options: {} },
  { name: "cff", since: "0.9.0", options: { controlFlowFlattening: true, controlFlowFlatteningThreshold: 1 } },
  { name: "dead", since: "0.10.0", options: { deadCodeInjection: true, deadCodeInjectionThreshold: 1 } },
  { name: "self", since: "0.0.0", options: { selfDefending: true } },
  { name: "split", since: "0.19.0", options: { splitStrings: true, splitStringsChunkLength: 3 } },
  { name: "keys", since: "0.14.0", options: { transformObjectKeys: true } },
  { name: "unicode", since: "0.0.0", options: { unicodeEscapeSequence: true } },
  { name: "mangled", since: "0.13.0", options: { identifierNamesGenerator: "mangled" } },
  { name: "numbers", since: "1.6.0", options: { numbersToExpressions: true } },
];

const WRAPPERS = [
  { name: "w0", since: "2.2.0", options: { stringArrayWrappersCount: 0 } },
  { name: "w1", since: "2.2.0", options: { stringArrayWrappersCount: 1, stringArrayWrappersChainedCalls: true } },
  { name: "w2u", since: "2.2.0", options: { stringArrayWrappersCount: 2, stringArrayWrappersChainedCalls: false } },
  { name: "w3f", since: "2.3.0", options: { stringArrayWrappersCount: 3, stringArrayWrappersType: "function" } },
  { name: "w2f5", since: "2.9.0", options: { stringArrayWrappersCount: 2, stringArrayWrappersType: "function", stringArrayWrappersParametersMaxCount: 5 } },
];

function parts(version) {
  return version.split(".").map(Number);
}

export function since(version, first) {
  const [a, b] = [parts(version), parts(first)];
  for (let i = 0; i < 3; i++) {
    if (a[i] !== b[i]) return a[i] > b[i];
  }
  return true;
}

const gated = (version, first, values) => (since(version, first) ? values : [NA]);

export const FACTORS = ["program", "encoding", "rotate", "shuffle", "shift", "indexes", "wrappers", "calls", "threshold", "extra", "container", "minifier"];

export function domain(version, factor, programs) {
  switch (factor) {
    case "program":
      return programs;
    case "encoding":
      return ["plain", "b64", "rc4"];
    case "rotate":
      return ["rot", "norot"];
    case "shuffle":
      return gated(version, "0.23.0", ["shuf", "noshuf"]);
    case "shift":
      return gated(version, "2.9.0", ["shift", "noshift"]);
    case "indexes":
      return gated(version, "2.9.0", ["inum", "istr"]);
    case "wrappers":
      return gated(version, "2.2.0", WRAPPERS.filter((w) => since(version, w.since)).map((w) => w.name));
    case "calls":
      return gated(version, "3.2.0", ["calls", "nocalls"]);
    case "threshold":
      return ["t75", "t1"];
    case "extra":
      return EXTRAS.filter((e) => since(version, e.since)).map((e) => e.name);
    case "container":
      return CONTAINERS;
    case "minifier":
      return MINIFIERS;
    default:
      throw new Error(`unknown factor ${factor}`);
  }
}

function encodingOption(version, encoding) {
  const name = { plain: "none", b64: "base64", rc4: "rc4" }[encoding];
  if (since(version, "2.0.0")) return name === "none" ? [] : [name];
  return name === "none" ? false : name;
}

export function obfuscatorOptions(version, row, seed) {
  const modern = since(version, "3.0.0");
  const options = {
    seed,
    stringArray: true,
    stringArrayThreshold: row.threshold === "t1" ? 1 : 0.75,
    stringArrayEncoding: encodingOption(version, row.encoding),
    [modern ? "stringArrayRotate" : "rotateStringArray"]: row.rotate === "rot",
  };
  if (row.shuffle !== NA) options[modern ? "stringArrayShuffle" : "shuffleStringArray"] = row.shuffle === "shuf";
  if (row.shift !== NA) options.stringArrayIndexShift = row.shift === "shift";
  if (row.indexes !== NA) options.stringArrayIndexesType = [row.indexes === "inum" ? "hexadecimal-number" : "hexadecimal-numeric-string"];
  if (row.wrappers !== NA) Object.assign(options, WRAPPERS.find((w) => w.name === row.wrappers).options);
  if (row.calls !== NA) Object.assign(options, row.calls === "calls" ? { stringArrayCallsTransform: true, stringArrayCallsTransformThreshold: 1 } : { stringArrayCallsTransform: false });
  return { ...options, ...EXTRAS.find((e) => e.name === row.extra).options };
}

export function cellName(version, row) {
  const shape = FACTORS.slice(1, 10)
    .map((factor) => row[factor])
    .filter((value) => value !== NA && value !== "none")
    .join("-");
  return `${version}--${shape}--${row.container}--${row.minifier}`;
}
