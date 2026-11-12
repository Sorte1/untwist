import crypto from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";

import { APPS, HERE, bun, OUT, REPO, WORK, copyTree, filesUnder, fresh, jsFiles, require, runCell, sizeOf, writeText } from "./lib.mjs";
import { webpack4, webpack5 } from "./webpack.mjs";
import { rolldown, rollup, vite } from "./rollup.mjs";
import { browserify, buns, esbuild, parcel } from "./others.mjs";
import { MINIFIERS, OBFUSCATORS } from "./post.mjs";

const createHash = crypto.createHash;
crypto.createHash = (algorithm, ...rest) => createHash(algorithm === "md4" ? "sha256" : algorithm, ...rest);

const MINIFY_BASES = ["webpack5-prod-node", "webpack4-prod-node", "rollup4-cjs29-iife", "esbuild0.28-iife", "browserify", "parcel-nohoist"];
const OBFUSCATE_BASES = ["rollup4-cjs29-iife", "webpack5-prod-node", "esbuild0.28-iife"];
const SOURCE_VARIANTS = ["webpack5-prod-node", "rollup4-cjs29-iife", "esbuild0.28-iife"];

const escape = (text) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const glob = (pattern, text) => new RegExp(`^${pattern.split("*").map(escape).join(".*")}$`).test(text);

function preset(app, modules) {
  const polyfills = app.polyfills ?? [];
  if (!polyfills.length) return { targets: app.targets ?? "chrome 60", useBuiltIns: false, modules };
  const kept = polyfills.map(escape).join("|");
  return {
    targets: app.targets ?? "chrome 60", useBuiltIns: "usage", corejs: "3.51", modules,
    exclude: [new RegExp(`^(?!(${kept})$)(es|esnext|web)\\.`)],
  };
}
const BABEL = { esm: (app) => preset(app, false), cjs: (app) => preset(app, "commonjs") };

function withSource(cell, source) {
  return { ...cell, name: `${cell.name}-${source}src`, source, flags: `${cell.flags} (Babel modules=commonjs source)` };
}

const bases = [...webpack5, ...webpack4, ...rollup, ...vite, ...rolldown, ...esbuild, ...buns, ...parcel, ...browserify];
const CELLS = [...bases, ...bases.filter((c) => SOURCE_VARIANTS.includes(c.name)).map((c) => withSource(c, "cjs"))];

function appConfig(name) {
  const file = join(APPS, name, "matrix.json");
  return existsSync(file) ? JSON.parse(readFileSync(file, "utf8")) : {};
}

function prepare(name, variant) {
  const babel = require("@babel/core");
  const app = appConfig(name);
  const from = join(APPS, name);
  const to = fresh(join(HERE, `.src-${name}-${variant}`));
  copyTree(from, to);
  for (const file of jsFiles(to)) {
    const out = babel.transformFileSync(file, {
      babelrc: false, configFile: false, sourceType: "unambiguous", cwd: HERE,
      presets: [[require.resolve("@babel/preset-env"), BABEL[variant](app)], ...(app.presets ?? []).map((p) => require.resolve(p))],
      plugins: (app.plugins ?? []).map((p) => (Array.isArray(p) ? [require.resolve(p[0]), p[1]] : require.resolve(p))),
    });
    writeFileSync(file, out.code + "\n");
  }
  return to;
}

function embedsPath(dir) {
  return filesUnder(dir).some((file) => {
    const text = readFileSync(file, "utf8");
    return text.includes(REPO) || text.includes(HERE) || text.includes(process.env.HOME);
  });
}

const LICENSES = {
  "core-js": "LICENSE", lodash: "LICENSE", webpack: "LICENSE", rollup: "LICENSE.md", "@rollup/plugin-commonjs": "LICENSE",
  vite: "LICENSE.md", rolldown: "LICENSE", esbuild: "LICENSE.md", parcel: "LICENSE", browserify: "LICENSE",
  "javascript-obfuscator": "LICENSE.BSD", terser: "LICENSE", "uglify-js": "LICENSE", "@swc/core": "LICENSE",
};

function licenses() {
  const dir = fresh(join(OUT, "licenses"));
  for (const [name, file] of Object.entries(LICENSES)) {
    copyFileSync(join(HERE, "node_modules", name, file), join(dir, `LICENSE-${name.replace("@", "").replace("/", "-")}`));
  }
}

const rows = [];
const sums = [];
const dropped = [];

function keep(app, cell, dir, { modules, entry }, expected) {
  const target = fresh(join(OUT, app, cell.name));
  copyTree(dir, target);
  writeText(join(OUT, "expected", app, `${cell.name}.stdout`), `exit=${expected.exit}\n${expected.stdout}`);
  rows.push([app, cell.name, cell.family, cell.tool(), cell.loader, entry.join(","), modules ?? "-", cell.flags].join("\t"));
  for (const file of filesUnder(target)) sums.push(`${crypto.createHash("sha256").update(readFileSync(file)).digest("hex")}  ${relative(OUT, file)}`);
  console.log(`${app}/${cell.name}\t${modules ?? "-"} modules\t${(sizeOf(target) / 1024).toFixed(1)} KiB`);
}

function observe(dir, cell, entry) {
  const ran = runCell(dir, entry, cell.loader);
  if (ran.exit !== 0 || !ran.stdout) {
    const error = ran.stderr.split("\n").filter((l) => /^[A-Za-z]*Error\b/.test(l)).slice(0, 1).join(" ").split(REPO).join(".");
    return { failed: `exit ${ran.exit}: ${error.slice(0, 200)}` };
  }
  return ran;
}

async function post(app, base, baseDir, kind, name, step) {
  const cell = { ...base, name: `${base.name}+${name}`, tool: () => `${base.tool()} + ${step.tool()}`, flags: `${base.flags} | ${kind} ${step.flags}` };
  const dir = fresh(join(WORK, "out", app, cell.name));
  copyTree(baseDir, dir);
  for (const file of jsFiles(dir)) writeFileSync(file, await step.run(readFileSync(file, "utf8")));
  return [cell, dir];
}

async function buildApp(app, only) {
  const config = appConfig(app);
  const wanted = (name) => (config.cells ?? ["*"]).some((p) => glob(p, name));
  const sources = { esm: prepare(app, "esm"), cjs: prepare(app, "cjs") };
  const built = {};
  for (const cell of CELLS) {
    if (!wanted(cell.name) || (only.length && !only.some((o) => cell.name.includes(o)))) continue;
    if (cell.available && !cell.available()) {
      console.log(`${app}/${cell.name}\tskipped: tool not installed; the committed fixture is kept`);
      continue;
    }
    const dir = fresh(join(WORK, "out", app, cell.name));
    let shape;
    try {
      const result = await cell.build({ app, srcDir: sources[cell.source ?? "esm"], dir });
      shape = typeof result === "object" && result !== null ? result : { modules: result, entry: ["index.js"] };
      if (!shape.entry.length) shape = { ...shape, entry: ["index.js"] };
    } catch (error) {
      dropped.push(`${app}/${cell.name}\tbuild failed: ${String(error.message ?? error).split("\n")[0]}`);
      continue;
    }
    built[cell.name] = { cell, dir, shape };
  }
  const queue = Object.values(built).map((b) => [b.cell, b.dir, b.shape]);
  const plans = [
    ["minify", MINIFIERS, config.minify ?? MINIFY_BASES, config.minifiers],
    ["obfuscate", OBFUSCATORS, config.obfuscate ?? OBFUSCATE_BASES, config.obfuscators],
  ];
  for (const [kind, steps, names, chosen] of plans) {
    for (const baseName of names) {
      const base = built[baseName];
      if (!base) continue;
      for (const [name, step] of Object.entries(steps)) {
        if (chosen && !chosen.includes(name)) continue;
        try {
          const [cell, dir] = await post(app, base.cell, base.dir, kind, name, step);
          queue.push([cell, dir, base.shape]);
        } catch (error) {
          dropped.push(`${app}/${baseName}+${name}\t${kind} failed: ${String(error.message ?? error).split("\n")[0]}`);
        }
      }
    }
  }
  for (const [cell, dir, shape] of queue) {
    const ran = observe(dir, cell, shape.entry);
    if (embedsPath(dir)) dropped.push(`${app}/${cell.name}\tembeds a local path`);
    else if (ran.failed) dropped.push(`${app}/${cell.name}\toriginal does not run: ${ran.failed}`);
    else keep(app, cell, dir, shape, ran);
  }
}

const only = process.argv.slice(2);
const apps = readdirSync(APPS).filter((a) => existsSync(join(APPS, a, "index.js"))).sort();
mkdirSync(WORK, { recursive: true });
if (!only.length) {
  if (bun() === null) throw new Error("a full run needs bun on PATH (it writes every cell, the bun ones included)");
  for (const dir of [...apps, "expected"]) fresh(join(OUT, dir));
  licenses();
}
for (const app of apps) await buildApp(app, only);
for (const line of dropped) console.log(`dropped ${line}`);
if (!only.length) {
  writeText(join(OUT, "MANIFEST"), "# app\tcell\tfamily\ttool\tloader\tentry\tsource_modules\tflags\n" + rows.join("\n") + "\n");
  writeText(join(OUT, "SHA256SUMS"), sums.join("\n") + "\n");
  writeText(join(OUT, "DROPPED"), dropped.join("\n") + (dropped.length ? "\n" : ""));
  console.log(`MANIFEST: ${rows.length} cells, ${(sizeOf(OUT) / 1024 / 1024).toFixed(2)} MiB`);
}
