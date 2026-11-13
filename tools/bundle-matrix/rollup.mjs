import { copyFileSync, mkdirSync } from "node:fs";
import { join, relative } from "node:path";

import { WORK, fresh, jsFiles, require, tool, version } from "./lib.mjs";

const real = (id) => !id.startsWith("\0") && !id.includes("?commonjs");

function countModules(outputs) {
  const ids = new Set();
  for (const chunk of outputs) {
    for (const id of chunk.moduleIds ?? Object.keys(chunk.modules ?? {})) if (real(id)) ids.add(id);
  }
  return ids.size;
}

const LINES = {
  2: { rollup: "rollup-2", resolve: "@rollup/plugin-node-resolve-13", json: "@rollup/plugin-json-4" },
  3: { rollup: "rollup-3", resolve: "@rollup/plugin-node-resolve", json: "@rollup/plugin-json" },
  4: { rollup: "rollup", resolve: "@rollup/plugin-node-resolve", json: "@rollup/plugin-json" },
};

const plugin = (name) => {
  const loaded = require(name);
  return loaded.default ?? loaded;
};

function rollupCell({ major, commonjs, format, strictRequires, split = false, loader = "node", suffix = "" }) {
  const line = LINES[major];
  const cjsAlias = commonjs === 29 ? "@rollup/plugin-commonjs" : `@rollup/plugin-commonjs-${commonjs}`;
  const strict = strictRequires === undefined ? "" : `-strict-${strictRequires}`;
  const name = `rollup${major}-cjs${commonjs}-${format}${split ? "-split" : ""}${strict}${suffix}`;
  return {
    name,
    family: { iife: "rollup", cjs: "rollup", es: "rollup", umd: "umd", amd: "amd", system: "systemjs" }[format],
    tool: () => `${tool("rollup", line.rollup)} ${tool("@rollup/plugin-commonjs", cjsAlias)}`,
    loader,
    flags: `--format=${format} node-resolve commonjs(${strictRequires === undefined ? "defaults" : `strictRequires=${strictRequires}`}) json ${split ? "chunks" : "inlineDynamicImports"}`,
    build: async (ctx) => {
      const { rollup } = require(line.rollup);
      const options = strictRequires === undefined ? {} : { strictRequires };
      const bundle = await rollup({
        input: join(ctx.srcDir, "index.js"),
        plugins: [plugin(line.resolve)({ preferBuiltins: true }), plugin(cjsAlias)(options), plugin(line.json)()],
        onwarn: () => {},
      });
      const named = format === "iife" || format === "umd";
      const { output } = await bundle.write({
        dir: ctx.dir, format, name: named ? "matrix" : undefined, inlineDynamicImports: !split, exports: "auto",
        entryFileNames: "index.js", chunkFileNames: "[name].chunk.js",
      });
      return countModules(output);
    },
  };
}

export const rollup = [
  rollupCell({ major: 2, commonjs: 21, format: "iife" }),
  rollupCell({ major: 2, commonjs: 22, format: "iife" }),
  rollupCell({ major: 2, commonjs: 22, format: "iife", strictRequires: true }),
  rollupCell({ major: 3, commonjs: 24, format: "iife" }),
  rollupCell({ major: 3, commonjs: 25, format: "iife" }),
  rollupCell({ major: 3, commonjs: 25, format: "cjs", split: true }),
  rollupCell({ major: 4, commonjs: 25, format: "iife" }),
  rollupCell({ major: 4, commonjs: 28, format: "iife" }),
  rollupCell({ major: 4, commonjs: 29, format: "iife" }),
  rollupCell({ major: 4, commonjs: 29, format: "iife", strictRequires: true }),
  rollupCell({ major: 4, commonjs: 29, format: "iife", strictRequires: "auto" }),
  rollupCell({ major: 4, commonjs: 29, format: "iife", strictRequires: false }),
  rollupCell({ major: 4, commonjs: 29, format: "cjs" }),
  rollupCell({ major: 4, commonjs: 29, format: "cjs", split: true }),
  rollupCell({ major: 4, commonjs: 29, format: "es", split: true }),
  rollupCell({ major: 4, commonjs: 29, format: "umd" }),
  rollupCell({ major: 4, commonjs: 29, format: "amd", loader: "amd" }),
  rollupCell({ major: 4, commonjs: 29, format: "system", loader: "systemjs" }),
];

function viteCell(major, format, minify) {
  const alias = major === 8 ? "vite" : `vite-${major}`;
  return {
    name: `vite${major}-${format}${minify ? "-min" : ""}`,
    family: "rollup",
    tool: () => tool("vite", alias),
    loader: "node",
    flags: `build.lib formats=[${format}] build.ssr ssr.noExternal build.commonjsOptions.include=[/./] minify=${minify}`,
    build: async (ctx) => {
      const { build } = await import(alias);
      const outDir = fresh(join(WORK, "vite-out"));
      const result = await build({
        configFile: false, logLevel: "silent", root: ctx.srcDir,
        ssr: { target: "node", noExternal: true },
        build: {
          ssr: true, outDir, emptyOutDir: true, minify, write: true,
          lib: { entry: join(ctx.srcDir, "index.js"), formats: [format], name: "matrix", fileName: () => "index.js" },
          commonjsOptions: { include: [/./] },
          rollupOptions: { output: { inlineDynamicImports: format === "iife", entryFileNames: "index.js", chunkFileNames: "[name].chunk.js" } },
        },
      });
      for (const file of jsFiles(outDir)) {
        const target = join(ctx.dir, relative(outDir, file));
        mkdirSync(join(target, ".."), { recursive: true });
        copyFileSync(file, target);
      }
      const outputs = [result].flat().flatMap((r) => r.output ?? []);
      return countModules(outputs);
    },
  };
}

export const vite = [4, 5, 6, 7, 8].flatMap((major) => [viteCell(major, "iife", false), viteCell(major, "es", true)]);

function rolldownCell(format, minify) {
  return {
    name: `rolldown-${format}${minify ? "-min" : ""}`,
    family: "rolldown",
    tool: () => `rolldown@${version("rolldown")}`,
    loader: "node",
    flags: `format=${format} platform=node minify=${minify}`,
    build: async (ctx) => {
      const { rolldown } = await import("rolldown");
      const bundle = await rolldown({ input: join(ctx.srcDir, "index.js"), platform: "node", logLevel: "silent" });
      const { output } = await bundle.write({
        dir: ctx.dir, format, minify, name: format === "iife" ? "matrix" : undefined,
        inlineDynamicImports: format === "iife", entryFileNames: "index.js", chunkFileNames: "[name].chunk.js",
      });
      return countModules(output);
    },
  };
}

export const rolldown = [rolldownCell("iife", false), rolldownCell("cjs", false), rolldownCell("esm", false), rolldownCell("esm", true)];
