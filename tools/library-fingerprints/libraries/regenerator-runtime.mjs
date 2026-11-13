import { aliasVersions } from "./_versions.mjs";

export default {
  id: "regenerator-runtime",
  label: "regenerator-runtime",
  versions: aliasVersions("regenerator-runtime"),
  entry(dir) {
    return `require("${dir}/runtime");\n`;
  },
  module(rel) {
    return rel === "runtime.js" ? "runtime" : null;
  },
  obfuscate: "latest",
};
