import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { TOOLS, entries } from "./libs.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const dependencies = { ...TOOLS };
for (const { name, version, alias, slug, root } of entries()) {
  if (root === ".") {
    dependencies[alias] = `npm:${name}@${version}`;
    continue;
  }
  mkdirSync(join(HERE, root), { recursive: true });
  const isolated = { name: `untwist-libcorpus-${slug.replace(/[^a-z0-9-]/g, "-")}`, private: true, dependencies: { [name]: version } };
  writeFileSync(join(HERE, root, "package.json"), `${JSON.stringify(isolated, null, 2)}\n`);
}
const sorted = Object.fromEntries(Object.entries(dependencies).sort(([a], [b]) => a.localeCompare(b)));
const pkg = {
  name: "untwist-libcorpus",
  private: true,
  description: "Pinned library versions and bundlers used by tools/libcorpus to build library fingerprint tables and to validate them on real bundles.",
  dependencies: sorted,
};
writeFileSync(join(HERE, "package.json"), `${JSON.stringify(pkg, null, 2)}\n`);
