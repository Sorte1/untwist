import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..", "..");
const OUT = join(ROOT, "fixtures", "unpack-rolldown");
const APP = join(HERE, "app");
const ENTRY = join(APP, "index.js");

const ROLLDOWNS = ["beta30", "rc9", "1.0", "1.1", "1.2", "1.3"];
const VITES = ["8.0", "8.1", "8.2", "8.3"];
const LATEST = "1.3";
const KEPT = ["EXPECT", "XFAIL"];

const version = (alias) => JSON.parse(readFileSync(join(HERE, "node_modules", alias, "package.json"), "utf8")).version;
function viteRolldown(alias) {
  const from = createRequire(join(HERE, "node_modules", `vite-${alias}`, "package.json"));
  return JSON.parse(readFileSync(from.resolve("rolldown/package.json"), "utf8")).version;
}

const sha256 = (file) => createHash("sha256").update(readFileSync(file)).digest("hex");

function jsFiles(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return jsFiles(path);
    return entry.name.endsWith(".js") ? [path] : [];
  });
}

function rolldownCells(alias) {
  const formats = [
    { format: "iife", split: false },
    { format: "cjs", split: false },
    { format: "cjs", split: true },
    { format: "esm", split: false },
    { format: "esm", split: true },
  ];
  const cells = formats.flatMap(({ format, split }) =>
    [false, true].map((minify) => ({ format, split, minify, keepNames: false, treeshake: true })),
  );
  if (alias === LATEST) {
    cells.push(
      { format: "iife", split: false, minify: false, keepNames: true, treeshake: true },
      { format: "iife", split: false, minify: true, keepNames: true, treeshake: true },
      { format: "esm", split: false, minify: true, keepNames: true, treeshake: true },
      { format: "iife", split: false, minify: false, keepNames: false, treeshake: false },
      { format: "cjs", split: false, minify: false, keepNames: false, treeshake: false },
      { format: "esm", split: false, minify: true, keepNames: false, treeshake: false },
      { format: "umd", split: false, minify: false, keepNames: false, treeshake: true },
    );
  }
  return cells.map((cell) => ({ ...cell, tool: "rolldown", alias }));
}

function cellName(cell) {
  const parts = [`${cell.tool}${cell.alias}`, cell.format];
  if (cell.split) parts.push("split");
  if (cell.minify) parts.push("min");
  if (cell.keepNames) parts.push("keepnames");
  if (!cell.treeshake) parts.push("notreeshake");
  return parts.join("-");
}

function cellFlags(cell) {
  return [
    `format=${cell.format}`,
    `codeSplitting=${cell.split}`,
    `minify=${cell.minify}`,
    `keepNames=${cell.keepNames}`,
    `treeshake=${cell.treeshake}`,
    "platform=node",
  ].join(" ");
}

async function buildRolldown(cell, dir) {
  const { rolldown } = await import(`rolldown-${cell.alias}`);
  const bundle = await rolldown({
    input: ENTRY,
    platform: "node",
    logLevel: "silent",
    treeshake: cell.treeshake,
  });
  const split = cell.alias === "beta30" ? { inlineDynamicImports: !cell.split } : { codeSplitting: cell.split };
  await bundle.write({
    dir,
    format: cell.format,
    minify: cell.minify,
    ...(cell.keepNames ? { keepNames: true } : {}),
    name: cell.format === "umd" ? "fixture" : undefined,
    entryFileNames: "index.js",
    chunkFileNames: "[name].chunk.js",
    ...split,
  });
}

function viteCells(alias) {
  const formats = [
    { format: "iife", split: false },
    { format: "cjs", split: false },
    { format: "cjs", split: true },
    { format: "es", split: false },
    { format: "es", split: true },
  ];
  return formats.flatMap(({ format, split }) =>
    [false, true].map((minify) => ({ tool: "vite", alias, format, split, minify, keepNames: false, treeshake: true })),
  );
}

async function buildVite(cell, dir) {
  const { build } = await import(`vite-${cell.alias}`);
  const work = mkdtempSync(join(tmpdir(), "untwist-vite-"));
  await build({
    configFile: false,
    logLevel: "silent",
    root: APP,
    ssr: { target: "node", noExternal: true },
    build: {
      ssr: true,
      outDir: work,
      emptyOutDir: true,
      minify: cell.minify,
      write: true,
      lib: { entry: ENTRY, formats: [cell.format], name: "fixture", fileName: () => "index.js" },
      rolldownOptions: {
        output: { codeSplitting: cell.split, entryFileNames: "index.js", chunkFileNames: "[name].chunk.js" },
      },
    },
  });
  for (const file of jsFiles(work)) {
    const target = join(dir, relative(work, file));
    mkdirSync(dirname(target), { recursive: true });
    copyFileSync(file, target);
  }
  rmSync(work, { recursive: true, force: true });
}

function printed(dir) {
  return execFileSync(process.execPath, ["--no-warnings", "index.js"], { cwd: dir, encoding: "utf8" });
}

async function main() {
  mkdirSync(OUT, { recursive: true });
  for (const entry of readdirSync(OUT)) {
    if (!KEPT.includes(entry)) rmSync(join(OUT, entry), { recursive: true, force: true });
  }
  mkdirSync(join(OUT, "expected"), { recursive: true });
  const cells = [...ROLLDOWNS.flatMap(rolldownCells), ...VITES.flatMap(viteCells)];
  const manifest = [];
  const seen = new Map();
  for (const cell of cells) {
    const name = cellName(cell);
    const dir = join(OUT, name);
    mkdirSync(dir, { recursive: true });
    const builder = cell.tool === "vite" ? buildVite : buildRolldown;
    await builder(cell, dir);
    const out = printed(dir);
    if (!out.includes("lazy 42 lazy-default")) throw new Error(`${name} did not run to the end:\n${out}`);
    const files = jsFiles(dir).sort();
    const digest = files.map((file) => `${relative(dir, file)}:${sha256(file)}`).join(",");
    if (seen.has(digest)) {
      rmSync(dir, { recursive: true, force: true });
      console.log(`${name}: same output as ${seen.get(digest)}, dropped`);
      continue;
    }
    seen.set(digest, name);
    writeFileSync(join(OUT, "expected", `${name}.stdout`), out);
    const tool = `${cell.tool}@${version(`${cell.tool}-${cell.alias}`)}${cell.tool === "vite" ? `+rolldown@${viteRolldown(cell.alias)}` : ""}`;
    for (const file of files) {
      const path = relative(OUT, file);
      const loader = path.endsWith("/index.js") ? "node" : "chunk";
      manifest.push([path, tool, loader, cellFlags(cell), sha256(file)].join("\t"));
    }
    console.log(`${name}: ${files.length} file(s)`);
  }
  const header = "# path\ttool\tloader\tflags\tsha256\n";
  writeFileSync(join(OUT, "MANIFEST"), header + manifest.join("\n") + "\n");
  mkdirSync(join(OUT, "licenses"), { recursive: true });
  copyFileSync(join(HERE, "node_modules", "rolldown-1.3", "LICENSE"), join(OUT, "licenses", "LICENSE-rolldown"));
  copyFileSync(join(HERE, "node_modules", "vite-8.3", "LICENSE.md"), join(OUT, "licenses", "LICENSE-vite"));
}

await main();
