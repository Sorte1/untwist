import { readdirSync } from "node:fs";
import { extname, join, relative } from "node:path";

const SKIP_DIRS = new Set([
  "test", "tests", "__tests__", "spec", "specs", "example", "examples", "docs", "doc", "bench", "benchmark",
  "benchmarks", "scripts", "bin", "typings", "types", "node_modules", "coverage", "fixtures", "perf", "demo",
  "demos", "website", "editor", "manual", "playground", "devtools", "flow-typed", "__mocks__",
]);
const KEEP = new Map([["three", ["examples/js", "examples/jsm"]]]);
const SKIP_UNDER = new Map([["three", ["examples/js/libs", "examples/jsm/libs"]]]);
const SKIP_FILES = /^(gruntfile|gulpfile|karma[.-]|jest\.|babel\.config|rollup\.config|webpack\.config|vite\.config|eslint|prettier|\.)|\.config\.(c|m)?js$/i;
const SOURCE_EXT = new Set([".js", ".mjs", ".cjs"]);

export function form(file, text) {
  if (/\.min\.(c|m)?js$/.test(file) || /[.-]min[.-]/.test(file)) return "min";
  const head = text.slice(0, 4000);
  if (/typeof\s+define\s*===?\s*['"]function['"]\s*&&\s*define\.amd/.test(head) && /typeof\s+(exports|module)\b/.test(head)) return "umd";
  if (file.endsWith(".mjs") || /^\s*(import\s*[\w{*'"]|export\s+[\w{*])/m.test(text)) return "esm";
  if (file.endsWith(".cjs") || /\brequire\s*\(|\bmodule\.exports\b|\bexports\.\w+\s*=/.test(text)) return "cjs";
  return "script";
}

export function* walk(root, dir, name, restricted = false) {
  const keep = KEEP.get(name) ?? [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const abs = join(dir, entry.name);
    const rel = relative(root, abs);
    if (entry.isDirectory()) {
      if ((SKIP_UNDER.get(name) ?? []).includes(rel)) continue;
      if (keep.some((path) => path === rel || rel.startsWith(`${path}/`))) {
        yield* walk(root, abs, name, false);
        continue;
      }
      if (keep.some((path) => path.startsWith(`${rel}/`))) {
        yield* walk(root, abs, name, true);
        continue;
      }
      if (restricted || SKIP_DIRS.has(entry.name.toLowerCase())) continue;
      yield* walk(root, abs, name, false);
    } else if (!restricted && entry.isFile() && SOURCE_EXT.has(extname(entry.name)) && !entry.name.endsWith(".d.ts") && !SKIP_FILES.test(entry.name)) {
      yield [abs, rel];
    }
  }
}
