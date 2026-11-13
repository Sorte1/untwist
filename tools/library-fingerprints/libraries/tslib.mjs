import { aliasVersions, filesIn } from "./_versions.mjs";

export default {
  id: "tslib",
  label: "tslib",
  versions: aliasVersions("tslib"),
  entry(dir) {
    const es6 = filesIn(dir, ".", /^tslib\.es6\.m?js$/)[0];
    const lines = [`const cjs = require("${dir}/tslib.js");`, "globalThis.__tslib = [cjs];"];
    if (es6) lines.push(`globalThis.__tslib.push(require("${dir}/${es6}"));`);
    return lines.join("\n") + "\n";
  },
  module(rel) {
    if (rel === "tslib.js") return "tslib";
    if (/^tslib\.es6\.m?js$/.test(rel)) return "tslib.es6";
    return null;
  },
  obfuscate: "latest",
};
