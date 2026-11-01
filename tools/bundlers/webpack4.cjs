const path = require("path");
const webpack = require("webpack");

const out = path.join(process.argv[2], "webpack");
const once = new webpack.optimize.LimitChunkCountPlugin({ maxChunks: 1 });

const base = {
  context: __dirname,
  entry: "./app/index.js",
  target: "node",
  node: false,
  output: { path: out, hashFunction: "sha256" },
  plugins: [once],
};

const builds = [
  { ...base, mode: "development", devtool: false, output: { ...base.output, filename: "webpack4-dev-paths.js" } },
  {
    ...base,
    mode: "production",
    externals: { path: "commonjs path" },
    optimization: { minimize: false },
    output: { ...base.output, filename: "webpack4-externals.js" },
  },
];

webpack(builds, (error, stats) => {
  if (error || stats.hasErrors()) {
    console.error(error || stats.toString("errors-only"));
    process.exit(1);
  }
});
