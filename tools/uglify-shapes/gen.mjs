import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = join(HERE, "../../fixtures/uglify-shapes");
const require = createRequire(join(HERE, "package.json"));
const uglify = require("uglify-js");
const version = JSON.parse(readFileSync(join(HERE, "node_modules/uglify-js/package.json"), "utf8")).version;

const VARIANTS = {
  min: { compress: true, mangle: true },
  top: { compress: { toplevel: true, passes: 2 }, mangle: { toplevel: true } },
};

const manifest = [];
for (const file of readdirSync(join(OUT, "src")).filter((f) => f.endsWith(".js")).sort()) {
  const code = readFileSync(join(OUT, "src", file), "utf8");
  for (const [variant, options] of Object.entries(VARIANTS)) {
    const result = uglify.minify(code, options);
    if (result.error) throw result.error;
    const name = file.replace(/\.js$/, `.${variant}.js`);
    writeFileSync(join(OUT, name), result.code + "\n");
    manifest.push(`${name}\tuglify-js@${version}\t${JSON.stringify(options)}`);
  }
}
writeFileSync(join(OUT, "MANIFEST"), manifest.join("\n") + "\n");
