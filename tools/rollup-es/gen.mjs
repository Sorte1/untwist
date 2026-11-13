import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = join(HERE, "..", "..", "fixtures", "unpack-rollup-es");
const KEPT = ["EXPECT", "XFAIL"];
const ENTRY = join(HERE, "entry.js");
const OTHER = join(HERE, "other.js");

const sha256 = (file) => createHash("sha256").update(readFileSync(file)).digest("hex");
const version = (alias) => JSON.parse(readFileSync(join(HERE, "node_modules", alias, "package.json"), "utf8")).version;

function jsFiles(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return jsFiles(path);
    return entry.name.endsWith(".js") ? [path] : [];
  });
}

const SHAPES = ["single", "split", "shared"];

async function minified(dir) {
  const { minify } = await import("terser");
  for (const file of jsFiles(dir)) {
    const result = await minify(readFileSync(file, "utf8"), { module: true });
    writeFileSync(file, result.code);
  }
}

async function buildRollup(cell, dir) {
  const { rollup } = await import(`rollup-${cell.major}`);
  const commonjs = (await import("@rollup/plugin-commonjs")).default;
  const json = (await import("@rollup/plugin-json")).default;
  const input = cell.shape === "shared" ? { index: ENTRY, other: OTHER } : { index: ENTRY };
  const bundle = await rollup({ input, plugins: [commonjs(), json()], external: [/^node:/], onwarn: () => {} });
  await bundle.write({
    dir,
    format: "es",
    inlineDynamicImports: cell.shape === "single",
    entryFileNames: "[name].js",
    chunkFileNames: "[name].chunk.js",
  });
  if (cell.minify) await minified(dir);
}

async function buildVite(cell, dir) {
  const { build } = await import(`vite-${cell.major}`);
  const work = mkdtempSync(join(tmpdir(), "untwist-vite-"));
  const entry = cell.shape === "shared" ? { index: ENTRY, other: OTHER } : { index: ENTRY };
  await build({
    configFile: false,
    logLevel: "silent",
    root: HERE,
    ssr: { target: "node", noExternal: true },
    build: {
      ssr: true,
      outDir: work,
      emptyOutDir: true,
      minify: cell.minify,
      write: true,
      lib: { entry, formats: ["es"], fileName: (_, name) => `${name}.js` },
      commonjsOptions: { include: [/./] },
      rollupOptions: {
        external: [/^node:/],
        output: { inlineDynamicImports: cell.shape === "single", entryFileNames: "[name].js", chunkFileNames: "[name].chunk.js" },
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

function cells() {
  const rollups = [3, 4].flatMap((major) =>
    SHAPES.flatMap((shape) => [false, true].map((minify) => ({ tool: "rollup", major, shape, minify }))),
  );
  const vites = [4, 5, 6, 7].flatMap((major) =>
    SHAPES.flatMap((shape) => [false, true].map((minify) => ({ tool: "vite", major, shape, minify }))),
  );
  return [...rollups, ...vites];
}

const name = (cell) => `${cell.tool}${cell.major}-es-${cell.shape}${cell.minify ? "-min" : ""}`;

function printed(dir) {
  return execFileSync(process.execPath, ["--no-warnings", "index.js"], { cwd: dir, encoding: "utf8" });
}

async function main() {
  mkdirSync(OUT, { recursive: true });
  for (const entry of readdirSync(OUT)) {
    if (!KEPT.includes(entry)) rmSync(join(OUT, entry), { recursive: true, force: true });
  }
  mkdirSync(join(OUT, "expected"), { recursive: true });
  const manifest = [];
  const seen = new Map();
  for (const cell of cells()) {
    const id = name(cell);
    const dir = join(OUT, id);
    mkdirSync(dir, { recursive: true });
    await (cell.tool === "vite" ? buildVite : buildRollup)(cell, dir);
    const out = printed(dir);
    if (!out.includes("lazy 42 lazy-default")) throw new Error(`${id} did not run to the end:\n${out}`);
    const files = jsFiles(dir).sort();
    const digest = files.map((file) => `${relative(dir, file)}:${sha256(file)}`).join(",");
    if (seen.has(digest)) {
      rmSync(dir, { recursive: true, force: true });
      console.log(`${id}: same output as ${seen.get(digest)}, dropped`);
      continue;
    }
    seen.set(digest, id);
    writeFileSync(join(OUT, "expected", `${id}.stdout`), out);
    const tool = `${cell.tool}@${version(`${cell.tool}-${cell.major}`)}`;
    const flags = `format=es shape=${cell.shape} minify=${cell.minify} commonjs@${version("@rollup/plugin-commonjs")}`;
    for (const file of files) {
      const path = relative(OUT, file);
      const loader = path.endsWith("/index.js") ? "node" : "chunk";
      manifest.push([path, tool, loader, flags, sha256(file)].join("\t"));
    }
    console.log(`${id}: ${files.length} file(s)`);
  }
  writeFileSync(join(OUT, "MANIFEST"), "# path\ttool\tloader\tflags\tsha256\n" + manifest.join("\n") + "\n");
  mkdirSync(join(OUT, "licenses"), { recursive: true });
  copyFileSync(join(HERE, "node_modules", "rollup-4", "LICENSE.md"), join(OUT, "licenses", "LICENSE-rollup"));
  copyFileSync(join(HERE, "node_modules", "vite-7", "LICENSE.md"), join(OUT, "licenses", "LICENSE-vite"));
}

await main();
