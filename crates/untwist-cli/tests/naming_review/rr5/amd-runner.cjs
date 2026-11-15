const fs = require("fs"), vm = require("vm"), path = require("path");
const out = []; const file = process.argv[2];
const mods = { path: require("path") };
const sandbox = { console: { log: (...a) => out.push(a.join(" ")) }, Error, define: function (deps, factory) { if (typeof deps === "function") { factory = deps; deps = []; } const exports = {}; const module = { exports }; const args = deps.map((d) => d === "exports" ? exports : d === "module" ? module : d === "require" ? (x) => mods[x] : mods[d]); const r = factory.apply(undefined, args); sandbox.result = r === undefined ? module.exports : r; } };
sandbox.define.amd = {};
vm.runInNewContext(fs.readFileSync(file, "utf8"), sandbox);
const N = (f) => Error.prototype.toString.call(f);
const r = sandbox.result; out.push("result " + (typeof r === "function" ? JSON.stringify(N(r)) : Object.keys(r).map((k) => k + "=" + JSON.stringify(typeof r[k] === "function" ? N(r[k]) : r[k])).join(",")));
console.log(out.join("\n"));
