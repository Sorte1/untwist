import { mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
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

async function bundles(entry, source) {
  const dir = join(CORPUS, entry.root, "node_modules", ".wide-check", entry.alias);
  mkdirSync(dir, { recursive: true });
  const file = join(dir, "index.js");
  writeFileSync(file, source);
  const { build } = require("esbuild");
  try {
    await build({ entryPoints: [file], bundle: true, write: false, platform: "browser", format: "iife", logLevel: "silent", external: BUILTINS, absWorkingDir: dir });
  } catch (error) {
    return `esbuild: ${String(error.message).split("\n").slice(0, 2).join(" ")}`;
  }
  const { rollup } = await import(pathToFileURL(require.resolve("rollup")).href);
  const commonjs = (await import(pathToFileURL(join(CORPUS, "node_modules/@rollup/plugin-commonjs/dist/es/index.js")).href)).default;
  const nodeResolve = (await import(pathToFileURL(join(CORPUS, "node_modules/@rollup/plugin-node-resolve/dist/es/index.js")).href)).default;
  try {
    const bundle = await rollup({ input: file, external: BUILTINS, onwarn: () => {}, plugins: [nodeResolve({ browser: true, preferBuiltins: false }), commonjs({ strictRequires: true })] });
    await bundle.generate({ format: "iife", inlineDynamicImports: true });
  } catch (error) {
    return `rollup: ${String(error.message).split("\n")[0]}`;
  }
  return null;
}

const [shard, shards] = (args[2] ?? "0/1").split("/").map(Number);
const specs = readdirSync(join(HERE, "libraries")).filter((name) => name.endsWith(".mjs") && only.test(name)).sort()
  .filter((_, index) => index % shards === shard);
const excludes = {};
const log = [];
for (const name of specs) {
  const spec = (await import(pathToFileURL(join(HERE, "libraries", name)).href)).default;
  for (const version of spec.versions) {
    const entry = entries().find((candidate) => candidate.alias === version.dir);
    const source = spec.entry(version.dir);
    const failure = await bundles(entry, source);
    if (!failure) {
      log.push([spec.id, version.version, "ok", source.split("\n").length - 2].join("\t"));
      continue;
    }
    const bad = [];
    const rels = source.split("\n").slice(1, -2).map((line) => /from "(.*)";$/.exec(line)?.[1]).filter(Boolean)
      .map((specifier) => specifier.replace(`${packageDir(entry)}/`, "").replace(`${entry.alias}/`, ""));
    const mainFails = await bundles(entry, `import * as m0 from ${JSON.stringify(entry.alias)};\nglobalThis.k = m0;\n`);
    for (const rel of rels) {
      if (await bundles(entry, entrySource(entry, [rel], { main: false }))) bad.push(rel);
    }
    excludes[entry.alias] = mainFails ? ["<main>", ...bad] : bad;
    const kept = rels.filter((rel) => !bad.includes(rel));
    const again = await bundles(entry, entrySource(entry, kept, { main: !mainFails }));
    log.push([spec.id, version.version, again ? `still failing: ${again}` : "fixed", kept.length, bad.length, mainFails ? "main-fails" : "", failure.slice(0, 160)].join("\t"));
    console.log(log.at(-1));
  }
}
const tag = args[1] ?? "";
writeFileSync(join(OUT, `wide-check${tag}.tsv`), `${log.join("\n")}\n`);
writeFileSync(join(OUT, `wide-excludes${tag}.json`), `${JSON.stringify(excludes, null, 1)}\n`);
