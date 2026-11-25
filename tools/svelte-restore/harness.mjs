import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "..", "..");
const CORPUS = join(HERE, "corpus");
const WORK = join(HERE, "work");
const require = createRequire(join(HERE, "package.json"));
const { compile } = require("svelte/compiler");
const UNTWIST = process.env.UNTWIST ?? join(REPO, "target", "release", "untwist");
const LANES = (process.env.LANES ?? "esm,bundle,min").split(",");
const ONLY = process.argv.slice(2);

const run = (command, args, options = {}) => spawnSync(command, args, { encoding: "utf8", maxBuffer: 1 << 28, ...options });

function compiled(source, filename) {
  return compile(source, { filename, generate: "client", css: "injected" }).js.code;
}

function linked(js, dir) {
  return js.replace(/(["'])\.\/([A-Za-z0-9_]+)\.svelte\1/g, (_, quote, name) => {
    const target = join(dir, `${name}.svelte.js`);
    if (!existsSync(target)) {
      writeFileSync(target, "");
      writeFileSync(target, linked(compiled(readFileSync(join(CORPUS, `${name}.svelte`), "utf8"), `${name}.svelte`), dir));
    }
    return `${quote}./${name}.svelte.js${quote}`;
  });
}

function render(js, specPath, dir, label) {
  const file = join(dir, `${label}.js`);
  writeFileSync(file, linked(js, dir));
  const result = run("untwist-run-slot", ["timeout", "30", "node", "--conditions=browser", join(HERE, "render.mjs"), file, specPath], { cwd: HERE });
  try {
    return JSON.parse(result.stdout);
  } catch {
    return { errors: [`render failed: ${(result.stderr || result.stdout).slice(0, 400)}`] };
  }
}

async function bundled(name, dir, minify) {
  const { rollup } = await import("rollup");
  const svelte = (await import("rollup-plugin-svelte")).default;
  const nodeResolve = (await import("@rollup/plugin-node-resolve")).default;
  const terser = (await import("@rollup/plugin-terser")).default;
  const entry = join(dir, "entry.js");
  writeFileSync(entry, `import Component from ${JSON.stringify(join(CORPUS, `${name}.svelte`))};\nwindow.__component = Component;\n`);
  const build = await rollup({
    input: entry,
    onwarn: () => {},
    plugins: [svelte({ compilerOptions: { css: "injected" }, emitCss: false }), nodeResolve({ browser: true, exportConditions: ["svelte"], rootDir: HERE })],
  });
  const { output } = await build.generate({ format: "iife", plugins: minify ? [terser()] : [] });
  return output[0].code;
}

function svelteFiles(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { withFileTypes: true, recursive: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith(".svelte"))
    .map((entry) => join(entry.parentPath ?? entry.path, entry.name));
}

function restored(input, dir, name) {
  const out = join(dir, "out");
  const result = run(UNTWIST, [input, "--restore", "svelte", "-o", out, "-f", "-v"]);
  if (result.status !== 0) return { error: `untwist exited ${result.status}: ${result.stderr.slice(0, 300)}` };
  const files = svelteFiles(out);
  const file = files.find((path) => basename(path) === `${name}.svelte`) ?? (files.length === 1 ? files[0] : undefined);
  if (!file) return { error: `no ${name}.svelte restored (${files.length} files); ${result.stderr.split("\n").filter((line) => line.includes("kept as JS")).slice(0, 3).join(" | ")}` };
  return { text: readFileSync(file, "utf8") };
}

const same = (left, right) => JSON.stringify(left) === JSON.stringify(right);

async function check(name, lane) {
  const dir = join(WORK, name, lane);
  mkdirSync(dir, { recursive: true });
  const source = readFileSync(join(CORPUS, `${name}.svelte`), "utf8");
  const specPath = existsSync(join(CORPUS, `${name}.json`)) ? join(CORPUS, `${name}.json`) : join(HERE, "empty.json");
  const original = compiled(source, `${name}.svelte`);
  const expected = render(original, specPath, dir, "original");
  if (expected.errors?.length) return { status: "harness", detail: expected.errors.join("; ") };
  const input = join(dir, "input.js");
  writeFileSync(input, lane === "esm" ? original : await bundled(name, dir, lane === "min"));
  const back = restored(input, dir, name);
  if (back.error) return { status: "not-restored", detail: back.error };
  writeFileSync(join(dir, `${name}.svelte`), back.text);
  let again;
  try {
    again = compiled(back.text, `${name}.svelte`);
  } catch (error) {
    return { status: "compile", detail: String(error.message).split("\n")[0] };
  }
  const actual = render(again, specPath, dir, "restored");
  if (actual.errors?.length) return { status: "render", detail: actual.errors.join("; ") };
  for (const key of ["snapshots", "logs", "css"]) {
    if (!same(expected[key], actual[key])) return { status: "mismatch", detail: `${key}: expected ${JSON.stringify(expected[key]).slice(0, 300)} got ${JSON.stringify(actual[key]).slice(0, 300)}` };
  }
  return { status: "pass" };
}

const FIXTURES = join(REPO, "crates", "untwist-svelte", "tests", "fixtures");

async function writeFixtures(names) {
  mkdirSync(FIXTURES, { recursive: true });
  for (const name of names) {
    writeFileSync(join(FIXTURES, `${name}.js`), compiled(readFileSync(join(CORPUS, `${name}.svelte`), "utf8"), `${name}.svelte`));
  }
  const dir = join(WORK, "fixture-bundle");
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(FIXTURES, "bundle.iife.js"), await bundled("Parent", dir, false));
}

writeFileSync(join(HERE, "empty.json"), "{}\n");
const names = readdirSync(CORPUS)
  .filter((file) => file.endsWith(".svelte"))
  .map((file) => file.slice(0, -".svelte".length))
  .filter((name) => ONLY.length === 0 || ONLY.includes(name))
  .sort();
if (process.env.FIXTURES) {
  await writeFixtures(names);
  process.exit(0);
}
const tally = {};
for (const lane of LANES) {
  let passed = 0;
  for (const name of names) {
    const result = await check(name, lane);
    if (result.status === "pass") passed += 1;
    console.log(`${lane.padEnd(6)} ${name.padEnd(14)} ${result.status}${result.detail ? `: ${result.detail}` : ""}`);
  }
  tally[lane] = `${passed}/${names.length}`;
}
console.log(Object.entries(tally).map(([lane, score]) => `${lane} ${score}`).join(", "));
