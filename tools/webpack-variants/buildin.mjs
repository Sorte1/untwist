import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../..");
const OUT = join(REPO, "fixtures/webpack-buildin");
const APP = join(HERE, "buildin-app");
const require = createRequire(join(HERE, "package.json"));
const ALIAS = "webpack-4";

const MODES = {
  dev: { mode: "development", optimization: { minimize: false } },
  prod: { mode: "production", optimization: { minimize: false } },
  min: { mode: "production", optimization: { minimize: true } },
};
const TARGETS = { web: "browser", node: "node" };
const APPS = { index: "./index.js", negative: "./negative.js", record: "./record.js" };

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
  rmSync(OUT, { recursive: true, force: true });
  mkdirSync(OUT, { recursive: true });
  const rows = [];
  for (const [app, entry] of Object.entries(APPS)) {
    for (const [mode, settings] of Object.entries(MODES)) {
      for (const [target, loader] of Object.entries(TARGETS)) {
        const name = `w4-${app}-${mode}-${target}`;
        const config = {
          context: APP,
          entry: { [name]: entry },
          target,
          mode: settings.mode,
          devtool: false,
          output: { path: OUT, filename: "[name].js" },
          optimization: settings.optimization,
          performance: { hints: false },
        };
        await compile(webpack, config);
        const file = join(OUT, `${name}.js`);
        writeFileSync(join(OUT, `${name}.stdout`), run(file, loader));
        const sha = createHash("sha256").update(readFileSync(file)).digest("hex");
        rows.push([name, `webpack@${version()}`, loader, `mode=${settings.mode} minimize=${settings.optimization.minimize} target=${target}`, sha].join("\t"));
        console.log(name);
      }
    }
  }
  writeFileSync(join(OUT, "MANIFEST"), "# cell\tbundler\tloader\tflags\tsha256\n" + rows.join("\n") + "\n");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
