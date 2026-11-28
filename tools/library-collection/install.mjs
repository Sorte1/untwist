import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const [, , dir, name, version, taken] = process.argv;
if (!dir || !name || !version) {
  console.error("usage: node install.mjs <dir> <package> <version> [<selection date>]");
  process.exit(2);
}
const DAY_MS = 24 * 60 * 60 * 1000;
const before = taken ? ["--before", new Date(Date.parse(taken) + DAY_MS).toISOString().slice(0, 10)] : [];
if (existsSync(join(dir, "node_modules", name, "package.json"))) process.exit(0);
mkdirSync(dir, { recursive: true });
writeFileSync(join(dir, "package.json"), JSON.stringify({ name: "untwist-library-collection-install", private: true, dependencies: { [name]: version } }, null, 1) + "\n");
execFileSync("npm", ["install", "--ignore-scripts", "--no-audit", "--no-fund", "--omit=optional", "--loglevel=error", "--package-lock=true", ...before], { cwd: dir, stdio: ["ignore", "ignore", "inherit"] });
