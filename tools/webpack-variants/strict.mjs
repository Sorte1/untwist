import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../..");
const OUT = join(REPO, "fixtures/webpack-strict");
const APP = join(HERE, "strict-app");
const LICENCE = join(REPO, "fixtures/webpack-eval/LICENSE-webpack");
const require = createRequire(join(HERE, "package.json"));
const ALIAS = "webpack";

const MODES = { none: "none", dev: "development" };
const TARGET = "web";
const LOADER = "browser";

function compile(webpack, config) {
  return new Promise((done, fail) => {
    webpack(config, (error, stats) => {
      if (error || stats.hasErrors()) return fail(error || new Error(stats.toString("errors-only")));
      done();
    });
  });
}

function version() {
  return JSON.parse(readFileSync(require.resolve(`${ALIAS}/package.json`), "utf8")).version;
}

function run(file, loader) {
  const out = spawnSync(process.execPath, ["--no-warnings", join(HERE, "run.mjs"), "--loader", loader, file], {
    cwd: dirname(file),
    encoding: "utf8",
    timeout: 20000,
  });
  if (out.status !== 0) throw new Error(`${file}: exit ${out.status}\n${out.stderr}`);
  return out.stdout;
}

async function main() {
  const webpack = require(ALIAS);
  const major = version().split(".")[0];
  rmSync(OUT, { recursive: true, force: true });
  mkdirSync(OUT, { recursive: true });
  const rows = [];
  for (const [mode, setting] of Object.entries(MODES)) {
    const name = `w${major}-${mode}`;
    await compile(webpack, {
      context: APP,
      entry: { [name]: "./index.js" },
      target: TARGET,
      mode: setting,
      devtool: false,
      output: { path: OUT, filename: "[name].js" },
      performance: { hints: false },
    });
    const file = join(OUT, `${name}.js`);
    if (readFileSync(file, "utf8").includes(REPO)) throw new Error(`${file} embeds a local path`);
    writeFileSync(join(OUT, `${name}.stdout`), run(file, LOADER));
    const sha = createHash("sha256").update(readFileSync(file)).digest("hex");
    rows.push([name, `webpack@${version()}`, LOADER, `mode=${setting} devtool=false target=${TARGET}`, sha].join("\t"));
    console.log(name);
  }
  copyFileSync(LICENCE, join(OUT, "LICENSE-webpack"));
  writeFileSync(join(OUT, "MANIFEST"), "# cell\tbundler\tloader\tflags\tsha256\n" + rows.join("\n") + "\n");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
