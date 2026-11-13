import { aliasVersions, filesIn } from "./_versions.mjs";

export default {
  id: "babel-runtime",
  label: "@babel/runtime",
  versions: aliasVersions("babel-runtime"),
  entry(dir) {
    const helpers = filesIn(dir, "helpers", /\.js$/).map((name) => `require("${dir}/helpers/${name.replace(/\.js$/, "")}")`);
    const regenerator = filesIn(dir, "regenerator", /^index\.js$/).map(() => `require("${dir}/regenerator")`);
    return `globalThis.__babelHelpers = [\n${[...helpers, ...regenerator].join(",\n")}\n];\n`;
  },
  module(rel) {
    const helper = /^helpers\/(?:esm\/)?([^/]+)\.m?js$/.exec(rel);
    if (helper) return `helpers/${helper[1]}`;
    return rel === "regenerator/index.js" ? "regenerator" : null;
  },
  obfuscate: "latest",
};
