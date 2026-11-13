import { appendFileSync, existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { builtinModules, createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { entries } from "../libcorpus/libs.mjs";
import { entrySource, files, packageDir } from "./wide.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const CORPUS = join(HERE, "../libcorpus");
const require = createRequire(join(CORPUS, "package.json"));
const OUT = process.env.UNTWIST_LIBCORPUS_OUT ?? join(process.env.HOME, ".cache/untwist-libcorpus");
const BUILTINS = [...builtinModules, ...builtinModules.map((name) => `node:${name}`)];
const args = process.argv.slice(2);
const only = new RegExp(args[0] ?? ".");

async function bundles(entry, source, which = "both") {
  const dir = join(CORPUS, entry.root, "node_modules", ".wide-check", entry.alias);
  mkdirSync(dir, { recursive: true });
  const file = join(dir, "index.js");
  writeFileSync(file, source);
  const { build } = require("esbuild");
  if (which !== "rollup") try {
    await build({ entryPoints: [file], bundle: true, write: false, platform: "browser", format: "iife", logLevel: "silent", absWorkingDir: dir });
  } catch (error) {
    return `esbuild: ${String(error.message).split("\n").slice(0, 2).join(" ")}`;
  }
  if (which === "esbuild") return null;
  const { rollup } = await import(pathToFileURL(require.resolve("rollup")).href);
  const commonjs = (await import(pathToFileURL(join(CORPUS, "node_modules/@rollup/plugin-commonjs/dist/es/index.js")).href)).default;
  const nodeResolve = (await import(pathToFileURL(join(CORPUS, "node_modules/@rollup/plugin-node-resolve/dist/es/index.js")).href)).default;
  try {
    const bundle = await rollup({ input: file, onwarn: () => {}, plugins: [nodeResolve({ preferBuiltins: false }), commonjs({ strictRequires: true })] });
    await bundle.generate({ format: "cjs", inlineDynamicImports: true });
  } catch (error) {
    return `rollup: ${String(error.message).split("\n")[0]}`;
  }
  return null;
}

const [shard, shards] = (args[2] ?? "0/1").split("/").map(Number);
const specs = readdirSync(join(HERE, "libraries")).filter((name) => name.endsWith(".mjs") && only.test(name) && readFileSync(join(HERE, "libraries", name), "utf8").includes("../wide.mjs")).sort();
let counter = -1;
const tag = args[1] ?? "";
const LOG = join(OUT, `wide-check${tag}.tsv`);
const EXCL = join(OUT, `wide-excludes${tag}.jsonl`);
const done = new Set(existsSync(LOG) ? readFileSync(LOG, "utf8").split("\n").filter(Boolean).map((line) => line.split("\t").slice(0, 2).join("@")) : []);
const record = (line, slug, list) => {
  appendFileSync(LOG, `${line}\n`);
  if (list) appendFileSync(EXCL, `${JSON.stringify([slug, list])}\n`);
  console.log(line);
};
for (const name of specs) {
  const spec = (await import(pathToFileURL(join(HERE, "libraries", name)).href)).default;
  if (!spec.versions?.[0]?.root) continue;
  for (const version of spec.versions) {
    counter += 1;
    if (counter % shards !== shard || done.has(`${spec.id}@${version.version}`)) continue;
    const entry = entries().find((candidate) => candidate.alias === version.dir && candidate.version === version.version);
    const source = spec.entry(version.dir, version);
    const failure = await bundles(entry, source);
    if (!failure) {
      record([spec.id, version.version, "ok", source.split("\n").length - 2].join("\t"));
      continue;
    }
    const bad = [];
    const rels = source.split("\n").slice(1, -2).map((line) => /require\("(.*)"\);$/.exec(line)?.[1]).filter(Boolean)
      .map((specifier) => specifier.replace(`${packageDir(entry)}/`, "").replace(`${entry.alias}/`, ""));
    const mainFails = await bundles(entry, `globalThis.k = require(${JSON.stringify(entry.alias)});\n`);
    for (const rel of rels) {
      if (await bundles(entry, entrySource(entry, [rel], { main: false }), "esbuild")) bad.push(rel);
    }
    if (await bundles(entry, entrySource(entry, rels.filter((rel) => !bad.includes(rel)), { main: !mainFails }), "rollup")) {
      for (const rel of rels) {
        if (!bad.includes(rel) && await bundles(entry, entrySource(entry, [rel], { main: false }), "rollup")) bad.push(rel);
      }
    }
    const list = mainFails ? ["<main>", ...bad] : bad;
    const kept = rels.filter((rel) => !bad.includes(rel));
    const again = await bundles(entry, entrySource(entry, kept, { main: !mainFails }));
    record([spec.id, version.version, again ? `still failing: ${again}` : "fixed", kept.length, bad.length, mainFails ? "main-fails" : "", failure.slice(0, 160)].join("\t"), entry.slug, list);
  }
}
