const Module = require("node:module");
const { readFileSync } = require("node:fs");

const BASE_CLASS_CHECK = /(targets|dependencies)\.length < baseClassDependencyCount/g;
const PLANNING = /[\\/]inversify[\\/]lib[\\/]planning[\\/](planner|reflection_utils)\.js$/;

const compileJs = Module._extensions[".js"];
Module._extensions[".js"] = function (module, filename) {
  if (!PLANNING.test(filename)) return compileJs(module, filename);
  module._compile(readFileSync(filename, "utf8").replace(BASE_CLASS_CHECK, "false"), filename);
};

const Obfuscator = require(process.argv[2]);
const jobs = JSON.parse(readFileSync(0, "utf8"));
const results = jobs.map(({ id, code, options }) => {
  try {
    return { id, code: Obfuscator.obfuscate(code, options).getObfuscatedCode() };
  } catch (error) {
    return { id, error: String(error && error.message).replace(/\s+/g, " ").slice(0, 300) };
  }
});
process.stdout.write(JSON.stringify(results));
