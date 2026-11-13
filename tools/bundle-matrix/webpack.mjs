import { require, tool } from "./lib.mjs";

const RUNTIME = /^(\(webpack\)|webpack\/runtime|external |webpack\/container|webpack\/sharing|provide shared|consume shared|container entry|remote )/;

const UNGROUPED = {
  groupModulesByPath: false, groupModulesByType: false, groupModulesByExtension: false, groupModulesByAttributes: false,
  groupModulesByCacheStatus: false, groupModulesByLayer: false, orphanModules: false, cachedModules: true, runtimeModules: false,
};

function counted(modules) {
  const names = new Set();
  const walk = (list) => {
    for (const m of list ?? []) {
      if (m.modules?.length) walk(m.modules);
      else if ((!m.type || m.type === "module") && m.name && !RUNTIME.test(m.name)) names.add(m.name);
    }
  };
  walk(modules);
  return names.size;
}

function compile(webpack, config) {
  return new Promise((done, fail) => {
    webpack(config, (error, stats) => {
      if (error || stats.hasErrors()) return fail(error || new Error(stats.toString("errors-only")));
      const json = stats.toJson({ all: false, modules: true, nestedModules: true, modulesSpace: Infinity, nestedModulesSpace: Infinity, maxModules: Infinity, entrypoints: true, ...UNGROUPED });
      const assets = (json.entrypoints.index?.assets ?? []).map((a) => a.name ?? a).filter((n) => n.endsWith(".js"));
      done({ modules: counted(json.modules), entry: assets });
    });
  });
}

function base(ctx, extra) {
  return {
    context: ctx.srcDir,
    entry: { index: "./index.js" },
    target: "node",
    node: false,
    devtool: false,
    output: { path: ctx.dir, filename: "[name].js", chunkFilename: "[name].chunk.js", publicPath: "" },
    resolve: { extensions: [".js", ".json"] },
    performance: { hints: false },
    ...extra,
  };
}

const W5 = { name: "webpack", alias: "webpack" };
const W4 = { name: "webpack", alias: "webpack-4" };

function cell(which, name, flags, config, loader = "node") {
  return {
    name,
    family: "webpack",
    tool: () => tool(which.name, which.alias),
    loader,
    flags,
    build: async (ctx) => {
      const webpack = require(which.alias);
      return compile(webpack, base(ctx, config(ctx, webpack)));
    },
  };
}

const noMin = { minimize: false };
const single = (webpack) => [new webpack.optimize.LimitChunkCountPlugin({ maxChunks: 1 })];

function federation(ctx, webpack) {
  const { ModuleFederationPlugin } = webpack.container;
  return {
    mode: "production",
    target: "web",
    optimization: noMin,
    plugins: [
      new ModuleFederationPlugin({
        name: "matrix",
        filename: "remoteEntry.js",
        exposes: { "./util": "./util.js" },
        shared: { "lodash/chunk": { eager: true, singleton: true, requiredVersion: false } },
      }),
    ],
  };
}

export const webpack5 = [
  cell(W5, "webpack5-dev-node", "mode=development devtool=false target=node", () => ({ mode: "development" })),
  cell(W5, "webpack5-dev-eval", "mode=development devtool=eval target=node", () => ({ mode: "development", devtool: "eval" })),
  cell(W5, "webpack5-dev-eval-source-map", "mode=development devtool=eval-cheap-module-source-map target=node", () => ({ mode: "development", devtool: "eval-cheap-module-source-map" })),
  cell(W5, "webpack5-prod-node", "mode=production minimize=false concatenateModules=false target=node", () => ({ mode: "production", optimization: { ...noMin, concatenateModules: false } })),
  cell(W5, "webpack5-prod-concat", "mode=production minimize=false target=node", () => ({ mode: "production", optimization: noMin })),
  cell(W5, "webpack5-prod-min", "mode=production target=node (terser default)", () => ({ mode: "production" })),
  cell(W5, "webpack5-web-split", "mode=production minimize=false target=web splitChunks=all", () => ({ mode: "production", target: "web", optimization: { ...noMin, splitChunks: { chunks: "all", minSize: 0 } } }), "browser"),
  cell(W5, "webpack5-web-runtime-chunk", "mode=production minimize=false target=web runtimeChunk=single", () => ({ mode: "production", target: "web", optimization: { ...noMin, runtimeChunk: "single" } }), "browser"),
  cell(W5, "webpack5-web-min", "mode=production target=web (terser default)", () => ({ mode: "production", target: "web" }), "browser"),
  cell(W5, "webpack5-web-noiife", "mode=production minimize=false target=web output.iife=false maxChunks=1", (ctx, webpack) => ({ mode: "production", target: "web", optimization: noMin, output: { ...base(ctx).output, iife: false }, plugins: single(webpack) }), "browser"),
  cell(W5, "webpack5-web-es5", "mode=production minimize=false target=[web,es5] maxChunks=1", (ctx, webpack) => ({ mode: "production", target: ["web", "es5"], optimization: noMin, plugins: single(webpack) }), "browser"),
  cell(W5, "webpack5-esm-output", "mode=production minimize=false experiments.outputModule output.module target=node", (ctx) => ({ mode: "production", experiments: { outputModule: true }, optimization: noMin, output: { ...base(ctx).output, module: true, chunkFormat: "module" } })),
  cell(W5, "webpack5-umd", "mode=production minimize=false library.type=umd target=node maxChunks=1", (ctx, webpack) => ({ mode: "production", optimization: noMin, output: { ...base(ctx).output, library: { name: "matrix", type: "umd" }, globalObject: "this" }, plugins: single(webpack) })),
  cell(W5, "webpack5-amd", "mode=production minimize=false library.type=amd target=node maxChunks=1", (ctx, webpack) => ({ mode: "production", optimization: noMin, output: { ...base(ctx).output, library: { name: "index", type: "amd" } }, plugins: single(webpack) }), "amd"),
  cell(W5, "webpack5-system", "mode=production minimize=false library.type=system target=node maxChunks=1", (ctx, webpack) => ({ mode: "production", optimization: noMin, output: { ...base(ctx).output, library: { type: "system" } }, plugins: single(webpack) }), "systemjs"),
  cell(W5, "webpack5-federation", "mode=production minimize=false target=web ModuleFederationPlugin(exposes ./util, shared eager)", federation, "browser"),
];

export const webpack4 = [
  cell(W4, "webpack4-dev-node", "mode=development devtool=false target=node output.pathinfo=false", (ctx) => ({ mode: "development", output: { ...base(ctx).output, pathinfo: false } })),
  cell(W4, "webpack4-dev-eval", "mode=development devtool=eval target=node output.pathinfo=false", (ctx) => ({ mode: "development", devtool: "eval", output: { ...base(ctx).output, pathinfo: false } })),
  cell(W4, "webpack4-prod-node", "mode=production minimize=false concatenateModules=false target=node", () => ({ mode: "production", optimization: { ...noMin, concatenateModules: false } })),
  cell(W4, "webpack4-prod-concat", "mode=production minimize=false target=node", () => ({ mode: "production", optimization: noMin })),
  cell(W4, "webpack4-prod-min", "mode=production target=node (terser default)", () => ({ mode: "production" })),
  cell(W4, "webpack4-web-split", "mode=production minimize=false target=web splitChunks=all", () => ({ mode: "production", target: "web", optimization: { ...noMin, splitChunks: { chunks: "all", minSize: 0 } } }), "browser"),
  cell(W4, "webpack4-umd", "mode=production minimize=false libraryTarget=umd target=node maxChunks=1", (ctx, webpack) => ({ mode: "production", optimization: noMin, output: { ...base(ctx).output, library: "matrix", libraryTarget: "umd", globalObject: "this" }, plugins: single(webpack) })),
];
