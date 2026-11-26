import { spawn } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { compiled } from "./compile.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "..", "..");
const CORPUS = join(HERE, "corpus");
const WORK = join(HERE, "work");
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

function linked(js, dir, sources) {
  return js.replace(/(["'])\.\/([A-Za-z0-9_-]+)\.jsx\1/g, (_, quote, name) => {
    const target = join(dir, `${name}.jsx.js`);
    if (!existsSync(target)) {
      writeFileSync(target, "");
      const from = sources.map((source) => join(source, `${name}.jsx`)).find(existsSync);
      writeFileSync(target, from ? linked(compiled(readFileSync(from, "utf8"), `${name}.jsx`), dir, sources) : "");
    }
    return `${quote}./${name}.jsx.js${quote}`;
  });
}

async function render(js, specPath, dir, sources) {
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const file = join(dir, "component.js");
  writeFileSync(file, linked(js, dir, sources));
  const result = await run("untwist-run-slot", ["timeout", "30", "node", "--conditions=browser", join(HERE, "render.mjs"), file, specPath], { cwd: HERE });
  try {
    return JSON.parse(result.stdout);
  } catch {
    return { errors: [`render failed: ${(result.stderr || result.stdout).slice(0, 400)}`] };
  }
}

const solidSource = {
  name: "solid-source",
  enforce: "pre",
  transform(code, id) {
    if (!id.endsWith(".jsx")) return null;
    return { code: compiled(code, basename(id)), map: null };
  },
};

function entryFile(names, dir) {
  const entry = join(dir, "entry.js");
  const imports = names.map((name, index) => `import C${index} from ${JSON.stringify(join(CORPUS, `${name}.jsx`))};`);
  const use =
    names.length === 1
      ? "render(() => createComponent(C0, {}), document.body);"
      : `window.__render = render;\nwindow.__component = [${names.map((_, index) => `C${index}`).join(", ")}];`;
  writeFileSync(entry, `import { render, createComponent } from "solid-js/web";\n${imports.join("\n")}\n${use}\n`);
  return entry;
}

async function rollupBuild(names, dir, minify, keepNames = false) {
  const { rollup } = await import("rollup");
  const nodeResolve = (await import("@rollup/plugin-node-resolve")).default;
  const terser = (await import("@rollup/plugin-terser")).default;
  const build = await rollup({
    input: entryFile(names, dir),
    onwarn: () => {},
    plugins: [solidSource, nodeResolve({ browser: true, exportConditions: ["solid", "browser"], rootDir: HERE })],
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
    plugins: [solidSource],
    resolve: { conditions: ["solid", "browser"] },
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

function jsxFiles(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { withFileTypes: true, recursive: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith(".jsx"))
    .map((entry) => join(entry.parentPath ?? entry.path, entry.name));
}

async function restoreAll(input, dir) {
  const out = join(dir, "out");
  rmSync(out, { recursive: true, force: true });
  const result = await run(UNTWIST, [input, "--restore", "solid", "-o", out, "-f", "-v"]);
  if (result.status !== 0) return { error: `untwist exited ${result.status}: ${result.stderr.slice(0, 300)}` };
  const kept = result.stderr.split("\n").filter((line) => line.includes("kept as JS"));
  return { out, files: jsxFiles(out), kept };
}

const TEMPLATE = /template\(\s*`((?:[^`\\]|\\.)*)`/g;

function templateKey(js) {
  return [...js.matchAll(TEMPLATE)].map((match) => match[1]).sort().join("\u0000");
}

const same = (left, right) => JSON.stringify(left) === JSON.stringify(right);
const specOf = (name) => (existsSync(join(CORPUS, `${name}.json`)) ? join(CORPUS, `${name}.json`) : join(HERE, "empty.json"));

async function compare(expected, text, name, dir, out) {
  let again;
  try {
    again = compiled(text, `${name}.jsx`);
  } catch (error) {
    return { status: "compile", detail: String(error.message).split("\n")[0] };
  }
  const actual = await render(again, specOf(name), join(dir, "restored"), [out, CORPUS]);
  if (actual.errors?.length) return { status: "render", detail: actual.errors.join("; ") };
  for (const key of ["snapshots", "logs"]) {
    if (!same(expected[key], actual[key])) return { status: "mismatch", detail: `${key}: expected ${JSON.stringify(expected[key]).slice(0, 300)} got ${JSON.stringify(actual[key]).slice(0, 300)}` };
  }
  return { status: "pass" };
}

async function expectation(name, dir) {
  const original = compiled(readFileSync(join(CORPUS, `${name}.jsx`), "utf8"), `${name}.jsx`);
  return { original, expected: await render(original, specOf(name), join(dir, "original"), [CORPUS]) };
}

async function check(name, lane) {
  const dir = join(WORK, name, lane);
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
  const file = back.files.find((path) => basename(path) === `${name}.jsx`) ?? back.files.find(keyed);
  if (!file) return { status: "not-restored", detail: `no ${name}.jsx restored (${back.files.length} files); ${back.kept.slice(0, 3).join(" | ")}` };
  return await compare(expected, readFileSync(file, "utf8"), name, dir, back.out);
}

async function checkApp(names, lane) {
  const dir = join(WORK, "app", lane);
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
    const byName = candidates.filter((candidate) => basename(candidate.path) === `${name}.jsx`);
    const matching = byName.length ? byName : candidates.filter((candidate) => candidate.key === templateKey(original));
    let result = { status: "not-restored", detail: `no restored component has its templates (${back.files.length} restored, ${back.kept.length} kept)` };
    for (const candidate of matching) {
      result = await compare(expected, candidate.text, name, own, back.out);
      if (result.status === "pass") break;
    }
    results.push([name, result]);
  }
  return results;
}

async function probe(names, label = "app") {
  const dir = join(WORK, "probe", label);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "terser.js"), await rollupBuild(names, dir, true, true));
  writeFileSync(join(dir, "esbuild.js"), await viteBuild(names, dir, true));
  writeFileSync(join(dir, "terser-mangled.js"), await rollupBuild(names, dir, true));
  writeFileSync(join(dir, "plain.js"), await rollupBuild(names, dir, false));
  writeFileSync(join(dir, "esbuild-mangled.js"), await viteBuild(names, dir));
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

writeFileSync(join(HERE, "empty.json"), "{}\n");
const names = readdirSync(CORPUS)
  .filter((file) => file.endsWith(".jsx"))
  .map((file) => file.slice(0, -".jsx".length))
  .filter((name) => ONLY.length === 0 || ONLY.includes(name))
  .sort();
const FIXTURES = join(REPO, "crates", "untwist-solid", "tests", "fixtures");

async function writeFixtures(names) {
  mkdirSync(FIXTURES, { recursive: true });
  for (const name of names) {
    writeFileSync(join(FIXTURES, `${name}.js`), compiled(readFileSync(join(CORPUS, `${name}.jsx`), "utf8"), `${name}.jsx`));
  }
  const dir = join(WORK, "fixture-bundle");
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(FIXTURES, "bundle.iife.js"), await rollupBuild(["Parent", "Context"], dir, false));
}

if (process.env.FIXTURES) {
  await writeFixtures(names);
  process.exit(0);
}
if (process.env.PROBE) {
  await probe(names);
  if (process.env.PROBE === "singles") await pooled(names, (name) => probe([name], name), 4);
  process.exit(0);
}

const tally = {};
for (const lane of LANES) {
  const results = lane.endsWith("-app") ? await checkApp(names, lane) : await pooled(names, async (name) => [name, await check(name, lane)]);
  let passed = 0;
  for (const [name, result] of results) {
    if (result.status === "pass") passed += 1;
    console.log(`${lane.padEnd(8)} ${name.padEnd(16)} ${result.status}${result.detail ? `: ${result.detail}` : ""}`);
  }
  tally[lane] = `${passed}/${names.length}`;
}
console.log("solid-js 1.9.19: " + Object.entries(tally).map(([lane, score]) => `${lane} ${score}`).join(", "));
