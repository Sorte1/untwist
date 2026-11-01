const path = require("path");
const Module = require("module");

const TERSER_PLUGIN = "next/dist/build/webpack/plugins/terser-webpack-plugin";
const resolve = Module._resolveFilename;
Module._resolveFilename = function (request, ...rest) {
  if (request.startsWith(TERSER_PLUGIN)) return __filename;
  return resolve.call(this, request, ...rest);
};

const compiled = require("next/dist/compiled/webpack/webpack.js");
compiled.init && compiled.init();
const webpack = compiled.webpack || compiled;
const { minify } = require("next/dist/compiled/terser");

class Terser {
  apply(compiler) {
    compiler.hooks.thisCompilation.tap("Terser", (compilation) => {
      compilation.hooks.processAssets.tapPromise(
        { name: "Terser", stage: webpack.Compilation.PROCESS_ASSETS_STAGE_OPTIMIZE_SIZE },
        async (assets) => {
          for (const name of Object.keys(assets).filter((n) => n.endsWith(".js"))) {
            const result = await minify(assets[name].source().toString(), {
              compress: { passes: 2 },
              mangle: true,
              format: { comments: false, wrap_func_args: false },
            });
            compilation.updateAsset(name, new webpack.sources.RawSource(result.code));
          }
        },
      );
    });
  }
}

const out = path.join(process.argv[2], "webpack");
const base = {
  context: __dirname,
  entry: "./app/index.js",
  target: "node",
  node: false,
  output: { path: out, hashFunction: "sha256" },
  plugins: [new webpack.optimize.LimitChunkCountPlugin({ maxChunks: 1 })],
};
const named = (filename) => ({ ...base.output, filename });
const minimized = { minimize: true, minimizer: [new Terser()] };

const builds = [
  { ...base, mode: "development", devtool: false, output: named("webpack5-dev.js") },
  { ...base, mode: "production", optimization: { minimize: false, concatenateModules: false }, output: named("webpack5-prod.js") },
  { ...base, mode: "production", optimization: { ...minimized, concatenateModules: false }, output: named("webpack5-prod-min.js") },
  { ...base, mode: "production", externals: { path: "commonjs path" }, optimization: { minimize: false }, output: named("webpack5-prod-concat.js") },
  { ...base, mode: "production", externals: { path: "commonjs path" }, optimization: minimized, output: named("webpack5-prod-concat-min.js") },
];

webpack(builds, (error, stats) => {
  if (error || stats.hasErrors()) {
    console.error(error || stats.toString("errors-only"));
    process.exit(1);
  }
  console.log(webpack.version);
});
