import { fork } from "node:child_process";
import { availableParallelism } from "node:os";
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = process.env.UNTWIST_LIBCORPUS_OUT ?? join(process.env.HOME, ".cache/untwist-libcorpus");
const BUNDLERS = ["webpack", "rollup", "esbuild", "vite"];
const OBFUSCATE_MAX_BYTES = 3 * 1024 * 1024;
export const TERSER_OPTIONS = { compress: { passes: 2 }, mangle: true, format: { comments: false } };
export const OBFUSCATOR_OPTIONS = { seed: 1, target: "browser" };

async function processOne(file, kind) {
  const source = readFileSync(file, "utf8");
  if (kind === "min") {
    const { minify } = await import("terser");
    const result = await minify(source, TERSER_OPTIONS);
    writeFileSync(file.replace(/\.js$/, ".min.js"), result.code);
    return;
  }
  const { default: obfuscator } = await import("javascript-obfuscator");
  writeFileSync(file.replace(/\.js$/, ".obf.js"), obfuscator.obfuscate(source, OBFUSCATOR_OPTIONS).getObfuscatedCode());
}

if (process.argv[2] === "--worker") {
  process.on("message", async ([file, kind]) => {
    try {
      await processOne(file, kind);
      process.send({ file, kind, ok: true });
    } catch (error) {
      process.send({ file, kind, ok: false, error: String(error.message ?? error).split("\n")[0].slice(0, 200) });
    }
  });
} else {
  const only = new RegExp(process.argv.includes("--only") ? process.argv[process.argv.indexOf("--only") + 1] : ".");
  const jobs = [];
  for (const id of readdirSync(join(OUT, "bundles")).sort()) {
    if (!only.test(id)) continue;
    for (const bundler of BUNDLERS) {
      const file = join(OUT, "bundles", id, `${bundler}.js`);
      if (!existsSync(file)) continue;
      jobs.push([file, "min"]);
      if (statSync(file).size <= OBFUSCATE_MAX_BYTES) jobs.push([file, "obf"]);
    }
  }
  jobs.sort((a, b) => statSync(b[0]).size - statSync(a[0]).size);
  const results = [];
  const workers = Math.min(jobs.length, Math.max(1, availableParallelism() - 4));
  await Promise.all(Array.from({ length: workers }, () => new Promise((resolve) => {
    const child = fork(fileURLToPath(import.meta.url), ["--worker"], { cwd: HERE, execArgv: ["--max-old-space-size=4096"] });
    const next = () => {
      const job = jobs.shift();
      if (!job) {
        child.kill();
        resolve();
        return;
      }
      child.send(job);
    };
    child.on("message", (message) => {
      results.push(message);
      if (!message.ok) console.log(`${message.kind} ${message.file}: ${message.error}`);
      next();
    });
    child.on("exit", (code) => {
      if (jobs.length && code) console.log(`worker exited ${code}`);
      resolve();
    });
    next();
  })));
  const failed = results.filter((result) => !result.ok);
  writeFileSync(join(OUT, "postprocess-report.tsv"), results.map((r) => [r.file, r.kind, r.ok ? "ok" : r.error].join("\t")).join("\n") + "\n");
  console.log(`${results.length - failed.length} ok, ${failed.length} failed`);
}
