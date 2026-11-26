import { spawn } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "..", "..");
const CORPUS = join(HERE, "corpus");
const WORK = join(HERE, "work");
const require = createRequire(join(HERE, "package.json"));
const VERSIONS = { "5.57.3": "svelte", "5.25.12": "svelte-5-25", "5.0.5": "svelte-5-0" };
const VERSION = process.env.SVELTE ?? "5.57.3";
const PKG = VERSIONS[VERSION];
if (!PKG) throw new Error(`SVELTE must be one of ${Object.keys(VERSIONS).join(", ")}`);
const { compile } = require(`${PKG}/compiler`);
const UNTWIST = process.env.UNTWIST ?? join(REPO, "target", "release", "untwist");
const LANES = (process.env.LANES ?? "esm,bundle,min,vite,min-app,vite-app").split(",");
const ONLY = process.argv.slice(2);

const run = (command, args, options = {}) =>
  new Promise((done) => {
    const child = spawn(command, args, { stdio: ["ignore", "pipe", "pipe"], ...options });
    const out = { stdout: "", stderr: "", status: null };
    child.stdout.on("data", (chunk) => (out.stdout += chunk));
    child.stderr.on("data", (chunk) => (out.stderr += chunk));
    child.on("error", (error) => (out.stderr += String(error)));
    child.on("close", (status) => done({ ...out, status }));
  });

function compiled(source, filename) {
  return compile(source, { filename, generate: "client", css: "injected" }).js.code;
}

const pinned = (js) => (PKG === "svelte" ? js : js.replace(/(["'])svelte(\/[^"']*)?\1/g, (_, quote, rest) => `${quote}${PKG}${rest ?? ""}${quote}`));

function linked(js, dir, sources) {
  return pinned(js).replace(/(["'])\.\/([A-Za-z0-9_-]+)\.svelte\1/g, (_, quote, name) => {
    const target = join(dir, `${name}.svelte.js`);
    if (!existsSync(target)) {
      writeFileSync(target, "");
      const from = sources.map((source) => join(source, `${name}.svelte`)).find(existsSync);
      writeFileSync(target, from ? linked(compiled(readFileSync(from, "utf8"), `${name}.svelte`), dir, sources) : "");
    }
    return `${quote}./${name}.svelte.js${quote}`;
  });
}

async function render(js, specPath, dir, sources) {
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const file = join(dir, "component.js");
  writeFileSync(file, linked(js, dir, sources));
  const result = await run("untwist-run-slot", ["timeout", "30", "node", "--conditions=browser", join(HERE, "render.mjs"), file, specPath], { cwd: HERE, env: { ...process.env, SVELTE_PKG: PKG } });
  try {
    return JSON.parse(result.stdout);
  } catch {
    return { errors: [`render failed: ${(result.stderr || result.stdout).slice(0, 400)}`] };
  }
}

const svelteSource = {
  name: "svelte-source",
  enforce: "pre",
  resolveId(id, importer) {
    if (PKG !== "svelte" && (id === "svelte" || id.startsWith("svelte/"))) return this.resolve(PKG + id.slice("svelte".length), importer, { skipSelf: true });
    return null;
  },
  transform(code, id) {
    if (!id.endsWith(".svelte")) return null;
    return compile(code, { filename: basename(id), generate: "client", css: "injected" }).js;
  },
};

function entryFile(names, dir) {
  const entry = join(dir, "entry.js");
  const imports = names.map((name, index) => `import C${index} from ${JSON.stringify(join(CORPUS, `${name}.svelte`))};`);
  const use =
    names.length === 1
      ? "mount(C0, { target: document.body });"
      : `window.__mount = mount;\nwindow.__component = [${names.map((_, index) => `C${index}`).join(", ")}];`;
  writeFileSync(entry, `import { mount } from "svelte";\n${imports.join("\n")}\n${use}\n`);
  return entry;
}

async function rollupBuild(names, dir, minify, keepNames = false) {
  const { rollup } = await import("rollup");
  const nodeResolve = (await import("@rollup/plugin-node-resolve")).default;
  const terser = (await import("@rollup/plugin-terser")).default;
  const build = await rollup({
    input: entryFile(names, dir),
    onwarn: () => {},
    plugins: [svelteSource, nodeResolve({ browser: true, exportConditions: ["svelte"], rootDir: HERE })],
  });
  const plugins = minify ? [terser(keepNames ? { mangle: { keep_fnames: true } } : {})] : [];
  const { output } = await build.generate({ format: "iife", plugins });
  return output[0].code;
}

async function viteBuild(names, dir, keepNames = false) {
  const { build } = await import("vite");
  const result = await build({
    configFile: false,
    root: dir,
    logLevel: "silent",
    plugins: [svelteSource],
    esbuild: keepNames ? { minifyIdentifiers: false } : {},
    build: { write: false, minify: "esbuild", modulePreload: false, rollupOptions: { input: entryFile(names, dir) } },
  });
  const outputs = (Array.isArray(result) ? result : [result]).flatMap((entry) => entry.output);
  return outputs.find((chunk) => chunk.type === "chunk" && chunk.isEntry).code;
}

const BUILDS = {
  bundle: (names, dir) => rollupBuild(names, dir, false),
  min: (names, dir) => rollupBuild(names, dir, true),
  vite: (names, dir) => viteBuild(names, dir),
};
BUILDS["min-app"] = BUILDS.min;
BUILDS["vite-app"] = BUILDS.vite;

function svelteFiles(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { withFileTypes: true, recursive: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith(".svelte"))
    .map((entry) => join(entry.parentPath ?? entry.path, entry.name));
}

async function restoreAll(input, dir) {
  const out = join(dir, "out");
  rmSync(out, { recursive: true, force: true });
  const result = await run(UNTWIST, [input, "--restore", "svelte", "-o", out, "-f", "-v"]);
  if (result.status !== 0) return { error: `untwist exited ${result.status}: ${result.stderr.slice(0, 300)}` };
  const kept = result.stderr.split("\n").filter((line) => line.includes("kept as JS"));
  return { out, files: svelteFiles(out), kept };
}

const TEMPLATE = /\$\.(?:from_html|from_svg|from_mathml|template|ns_template)\(`((?:[^`\\]|\\.)*)`/g;

function templateKey(js) {
  return [...js.matchAll(TEMPLATE)].map((match) => match[1]).sort().join("\u0000") + (js.includes("$.append_styles(") ? "\u0001css" : "");
}

const squeeze = (css) => String(css).replace(/\s+/g, " ").replace(/ ?([{};:,]) ?/g, "$1").replace(/;}/g, "}").trim();
const same = (left, right, key) => (key === "css" ? squeeze(left) === squeeze(right) : JSON.stringify(left) === JSON.stringify(right));
const specOf = (name) => (existsSync(join(CORPUS, `${name}.json`)) ? join(CORPUS, `${name}.json`) : join(HERE, "empty.json"));

async function compare(expected, text, name, dir, out) {
  let again;
  try {
    again = compiled(text, `${name}.svelte`);
  } catch (error) {
    return { status: "compile", detail: String(error.message).split("\n")[0] };
  }
  const actual = await render(again, specOf(name), join(dir, "restored"), [out, CORPUS]);
  if (actual.errors?.length) return { status: "render", detail: actual.errors.join("; ") };
  for (const key of ["snapshots", "logs", "css"]) {
    if (!same(expected[key], actual[key], key)) return { status: "mismatch", detail: `${key}: expected ${JSON.stringify(expected[key]).slice(0, 300)} got ${JSON.stringify(actual[key]).slice(0, 300)}` };
  }
  return { status: "pass" };
}

async function expectation(name, dir) {
  const original = compiled(readFileSync(join(CORPUS, `${name}.svelte`), "utf8"), `${name}.svelte`);
  return { original, expected: await render(original, specOf(name), join(dir, "original"), [CORPUS]) };
}

async function check(name, lane) {
  const dir = join(WORK, VERSION, name, lane);
  mkdirSync(dir, { recursive: true });
  const { original, expected } = await expectation(name, dir);
  if (expected.errors?.length) return { status: "harness", detail: expected.errors.join("; ") };
  const input = join(dir, "input.js");
  writeFileSync(input, lane === "esm" ? original : await BUILDS[lane]([name], dir));
  const back = await restoreAll(input, dir);
  if (back.error) return { status: "not-restored", detail: back.error };
  const keyed = (path) => {
    try {
      return templateKey(compiled(readFileSync(path, "utf8"), basename(path))) === templateKey(original);
    } catch {
      return false;
    }
  };
  const file = back.files.find((path) => basename(path) === `${name}.svelte`) ?? back.files.find(keyed) ?? (back.files.length === 1 ? back.files[0] : undefined);
  if (!file) return { status: "not-restored", detail: `no ${name}.svelte restored (${back.files.length} files); ${back.kept.slice(0, 3).join(" | ")}` };
  return await compare(expected, readFileSync(file, "utf8"), name, dir, back.out);
}

async function checkApp(names, lane) {
  const dir = join(WORK, VERSION, "app", lane);
  mkdirSync(dir, { recursive: true });
  const input = join(dir, "input.js");
  writeFileSync(input, await BUILDS[lane](names, dir));
  const back = await restoreAll(input, dir);
  if (back.error) return names.map((name) => [name, { status: "not-restored", detail: back.error }]);
  const candidates = back.files.map((path) => {
    const text = readFileSync(path, "utf8");
    try {
      return { path, text, key: templateKey(compiled(text, basename(path))) };
    } catch {
      return { path, text, key: null };
    }
  });
  const results = [];
  for (const name of names) {
    const own = join(dir, name);
    mkdirSync(own, { recursive: true });
    const { original, expected } = await expectation(name, own);
    if (expected.errors?.length) {
      results.push([name, { status: "harness", detail: expected.errors.join("; ") }]);
      continue;
    }
    const key = templateKey(original);
    const matching = candidates.filter((candidate) => candidate.key === key);
    let result = { status: "not-restored", detail: `no restored component has its templates (${back.files.length} restored, ${back.kept.length} kept)` };
    for (const candidate of matching) {
      result = await compare(expected, candidate.text, name, own, back.out);
      if (result.status === "pass") {
        candidate.claimed = true;
        break;
      }
    }
    results.push([name, result, expected, own]);
  }
  for (const entry of results) {
    const [name, result, expected, own] = entry;
    if (result.status !== "not-restored" || !expected) continue;
    for (const candidate of candidates.filter((candidate) => !candidate.claimed && !candidates.some((other) => other !== candidate && other.key === candidate.key && other.claimed))) {
      const tried = await compare(expected, candidate.text, name, own, back.out);
      if (tried.status === "pass") {
        candidate.claimed = true;
        entry[1] = tried;
        break;
      }
    }
  }
  return results.map(([name, result]) => [name, result]);
}

async function probe(names, label = "app") {
  const dir = join(WORK, "probe", VERSION, label);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "terser.js"), await rollupBuild(names, dir, true, true));
  writeFileSync(join(dir, "esbuild.js"), await viteBuild(names, dir, true));
  writeFileSync(join(dir, "terser-mangled.js"), await rollupBuild(names, dir, true));
  writeFileSync(join(dir, "plain.js"), await rollupBuild(names, dir, false));
  writeFileSync(join(dir, "esbuild-mangled.js"), await viteBuild(names, dir));
}

const FIXTURES = join(REPO, "crates", "untwist-svelte", "tests", "fixtures");

async function writeFixtures(names) {
  mkdirSync(FIXTURES, { recursive: true });
  for (const name of names) {
    writeFileSync(join(FIXTURES, `${name}.js`), compiled(readFileSync(join(CORPUS, `${name}.svelte`), "utf8"), `${name}.svelte`));
  }
  const dir = join(WORK, "fixture-bundle");
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(FIXTURES, "bundle.iife.js"), await rollupBuild(["Parent"], dir, false));
  const app = ["Parent", "Counter", "EachNested", "Boundary", "SpreadChild", "Todo"];
  for (const [label, build] of [["terser", () => rollupBuild(app, dir, true)], ["vite", () => viteBuild(app, dir)]]) {
    const input = join(dir, `${label}.js`);
    writeFileSync(input, await build());
    const out = join(dir, `${label}.out`);
    const result = await run(UNTWIST, [input, "-o", out, "-f"]);
    if (result.status !== 0) throw new Error(`untwist failed on the ${label} fixture: ${result.stderr}`);
    writeFileSync(join(FIXTURES, `bundle.${label}.untwisted.js`), readFileSync(join(out, "deobfuscated.js"), "utf8"));
    mkdirSync(join(FIXTURES, "raw"), { recursive: true });
    writeFileSync(join(FIXTURES, "raw", `bundle.${label}.js`), readFileSync(input, "utf8"));
  }
}

writeFileSync(join(HERE, "empty.json"), "{}\n");
const compiles = (name) => {
  try {
    compiled(readFileSync(join(CORPUS, `${name}.svelte`), "utf8"), `${name}.svelte`);
    return true;
  } catch {
    return false;
  }
};
const names = readdirSync(CORPUS)
  .filter((file) => file.endsWith(".svelte"))
  .map((file) => file.slice(0, -".svelte".length))
  .filter((name) => ONLY.length === 0 || ONLY.includes(name))
  .sort();
const skipped = names.filter((name) => !compiles(name));
if (skipped.length) {
  console.log(`svelte ${VERSION} cannot compile ${skipped.join(", ")}; left out`);
  names.splice(0, names.length, ...names.filter((name) => !skipped.includes(name)));
}
if (process.env.FIXTURES) {
  await writeFixtures(names);
  process.exit(0);
}
if (process.env.PROBE) {
  await probe(names);
  if (process.env.PROBE === "singles") {
    await pooled(names, (name) => probe([name], name), 4);
    const groups = [7, 12].flatMap((size) =>
      Array.from({ length: size }, (_, at) => [`group-${size}-${at}`, names.filter((_, index) => index % size === at)]),
    );
    await pooled(groups, ([label, group]) => probe(group, label), 4);
  }
  process.exit(0);
}
async function pooled(items, task, width = 6) {
  const results = new Array(items.length);
  let next = 0;
  const worker = async () => {
    while (next < items.length) {
      const index = next++;
      results[index] = await task(items[index]);
    }
  };
  await Promise.all(Array.from({ length: width }, worker));
  return results;
}

const tally = {};
for (const lane of LANES) {
  const results = lane.endsWith("-app") ? await checkApp(names, lane) : await pooled(names, async (name) => [name, await check(name, lane)]);
  let passed = 0;
  for (const [name, result] of results) {
    if (result.status === "pass") passed += 1;
    console.log(`${lane.padEnd(8)} ${name.padEnd(14)} ${result.status}${result.detail ? `: ${result.detail}` : ""}`);
  }
  tally[lane] = `${passed}/${names.length}`;
}
console.log(`svelte ${VERSION}: ` + Object.entries(tally).map(([lane, score]) => `${lane} ${score}`).join(", "));
