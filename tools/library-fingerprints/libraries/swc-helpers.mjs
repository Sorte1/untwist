import { aliasVersions, filesIn } from "./_versions.mjs";

function folder(dir) {
  for (const candidate of ["cjs", "lib"]) {
    if (filesIn(dir, candidate, /\.c?js$/).length) return candidate;
  }
  return null;
}

export default {
  id: "swc-helpers",
  label: "@swc/helpers",
  versions: aliasVersions("swc-helpers"),
  entry(dir) {
    const where = folder(dir);
    const files = where ? filesIn(dir, where, /^_.*\.c?js$/) : [];
    return `globalThis.__swcHelpers = [\n${files.map((name) => `require("${dir}/${where}/${name}")`).join(",\n")}\n];\n`;
  },
  module(rel) {
    const match = /^(?:cjs|lib|esm|_)\/(?:[^/]+\/)?(_[^/]+?)\.(?:c|m)?js$/.exec(rel);
    return match ? match[1].replace(/^_/, "") : null;
  },
  obfuscate: "latest",
};
