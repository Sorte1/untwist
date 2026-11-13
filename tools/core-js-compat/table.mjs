import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = dirname(fileURLToPath(import.meta.url));
const MODULES = join(ROOT, "node_modules");

const YEAR_ENGINES = ["chrome", "edge", "firefox", "safari", "ios", "node", "opera", "samsung"];

const SYNTAX = {
  2015: [
    "transform-template-literals",
    "transform-literals",
    "transform-function-name",
    "transform-arrow-functions",
    "transform-block-scoped-functions",
    "transform-classes",
    "transform-object-super",
    "transform-shorthand-properties",
    "transform-duplicate-keys",
    "transform-computed-properties",
    "transform-for-of",
    "transform-sticky-regex",
    "transform-unicode-escapes",
    "transform-unicode-regex",
    "transform-spread",
    "transform-destructuring",
    "transform-block-scoping",
    "transform-typeof-symbol",
    "transform-new-target",
    "transform-regenerator",
    "transform-parameters",
  ],
  2016: ["transform-exponentiation-operator"],
  2017: ["transform-async-to-generator"],
  2018: [
    "transform-async-generator-functions",
    "transform-object-rest-spread",
    "transform-dotall-regex",
    "transform-unicode-property-regex",
    "transform-named-capturing-groups-regex",
  ],
  2019: ["transform-optional-catch-binding", "transform-json-strings"],
  2020: ["transform-nullish-coalescing-operator", "transform-optional-chaining", "transform-export-namespace-from"],
  2021: ["transform-logical-assignment-operators", "transform-numeric-separator"],
  2022: [
    "transform-class-properties",
    "transform-private-methods",
    "transform-class-static-block",
    "transform-private-property-in-object",
  ],
  2023: [],
  2024: ["transform-unicode-sets-regex"],
};

const BUILT_INS = {
  2016: ["es.array.includes"],
  2017: [
    "es.object.values",
    "es.object.entries",
    "es.object.get-own-property-descriptors",
    "es.string.pad-start",
    "es.string.pad-end",
  ],
  2018: ["es.promise.finally"],
  2019: [
    "es.array.flat",
    "es.array.flat-map",
    "es.object.from-entries",
    "es.string.trim-start",
    "es.string.trim-end",
    "es.symbol.description",
  ],
  2020: ["es.string.match-all", "es.promise.all-settled", "es.global-this"],
  2021: ["es.string.replace-all", "es.promise.any", "es.aggregate-error"],
  2022: ["es.array.at", "es.string.at-alternative", "es.object.has-own", "es.error.cause"],
  2023: [
    "es.array.find-last",
    "es.array.find-last-index",
    "es.array.to-reversed",
    "es.array.to-sorted",
    "es.array.to-spliced",
    "es.array.with",
  ],
  2024: [
    "es.object.group-by",
    "es.map.group-by",
    "es.promise.with-resolvers",
    "es.string.is-well-formed",
    "es.string.to-well-formed",
    "es.array-buffer.transfer",
    "es.array-buffer.transfer-to-fixed-length",
    "es.array-buffer.detached",
  ],
};

const FIRST_YEAR = 2015;
const LAST_YEAR = 2024;
const OFFERED_FROM = 2017;

function key(version) {
  return String(version).split(/[.\-+]/).map((part) => Number.parseInt(part, 10) || 0);
}

function compare(a, b) {
  const [x, y] = [key(a), key(b)];
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    const d = (x[i] ?? 0) - (y[i] ?? 0);
    if (d !== 0) return d;
  }
  return 0;
}

function maxVersion(a, b) {
  if (a === undefined) return b;
  return compare(a, b) >= 0 ? a : b;
}

function readJson(...parts) {
  return JSON.parse(readFileSync(join(MODULES, ...parts), "utf8"));
}

function compatVersions() {
  return readdirSync(MODULES)
    .filter((dir) => /^core-js-compat-3\.\d+$/.test(dir))
    .map((dir) => ({ dir, version: readJson(dir, "package.json").version }))
    .sort((a, b) => compare(a.version, b.version));
}

function ranges(indices, versions) {
  const out = [];
  let start = null;
  let previous = null;
  for (const index of indices) {
    if (start !== null && index === previous + 1) {
      previous = index;
      continue;
    }
    if (start !== null) out.push([start, previous]);
    start = index;
    previous = index;
  }
  if (start !== null) out.push([start, previous]);
  return out
    .map(([low, high]) => (low === high ? versions[low] : `${versions[low]}~${versions[high]}`))
    .join(",");
}

function minVersion(a, b) {
  if (a === undefined) return b;
  return compare(a, b) <= 0 ? a : b;
}

function shipped(datasets) {
  const out = {};
  for (const data of datasets) {
    for (const [module, engines] of Object.entries(data)) {
      out[module] ??= {};
      for (const [engine, version] of Object.entries(engines)) {
        out[module][engine] = minVersion(out[module][engine], version);
      }
    }
  }
  return out;
}

function yearBaselines(plugins, first) {
  const lines = [];
  const need = {};
  for (let year = FIRST_YEAR; year <= LAST_YEAR; year++) {
    for (const plugin of SYNTAX[year] ?? []) {
      const data = plugins[plugin];
      if (!data) throw new Error(`@babel/compat-data has no ${plugin}`);
      for (const engine of YEAR_ENGINES) {
        if (!data[engine]) throw new Error(`${plugin} has no ${engine} version`);
        need[engine] = maxVersion(need[engine], data[engine]);
      }
    }
    for (const module of BUILT_INS[year] ?? []) {
      const data = first[module];
      if (!data) throw new Error(`core-js-compat has no ${module}`);
      for (const engine of YEAR_ENGINES) {
        if (!data[engine]) throw new Error(`${module} has no ${engine} version`);
        need[engine] = maxVersion(need[engine], data[engine]);
      }
    }
    if (year >= OFFERED_FROM) {
      const engines = YEAR_ENGINES.map((engine) => `${engine}:${need[engine]}`).join(",");
      lines.push(`#baseline\tes${year}\t${engines}`);
    }
  }
  return lines;
}

function main() {
  const compat = compatVersions();
  const versions = compat.map((entry) => entry.version);
  const rows = new Map();
  const datasets = compat.map((entry) => readJson(entry.dir, "data.json"));
  datasets.forEach((data, index) => {
    for (const [module, engines] of Object.entries(data)) {
      for (const [engine, version] of Object.entries(engines)) {
        const id = `${module}\t${engine}\t${version}`;
        if (!rows.has(id)) rows.set(id, []);
        rows.get(id).push(index);
      }
    }
  });
  const plugins = readJson("@babel", "compat-data", "data", "plugins.json");
  const babel = readJson("@babel", "compat-data", "package.json").version;
  const out = ["#untwist-core-js-compat\tv1"];
  out.push(`#source\tcore-js-compat ${versions[0]}~${versions[versions.length - 1]}, @babel/compat-data ${babel}`);
  out.push(`#versions\t${versions.join(",")}`);
  out.push(...yearBaselines(plugins, shipped(datasets)));
  const body = [...rows.entries()]
    .map(([id, indices]) => `${id}\t${ranges(indices, versions)}`)
    .sort();
  out.push(...body);
  process.stdout.write(`${out.join("\n")}\n`);
}

main();
