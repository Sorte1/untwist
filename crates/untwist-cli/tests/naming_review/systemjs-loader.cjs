const reg = {}, mods = {};
globalThis.System = { register(n, d, f) { reg[n] = [d, f]; } };
require("./systemjs-export-call.js");
function load(n) { if (n === "path") return { default: require("path") }; if (mods[n]) return mods[n]; const ns = mods[n] = {}; const [d, f] = reg[n]; const m = f((k, v) => { if (typeof k === "object") Object.assign(ns, k); else ns[k] = v; return v; }, { import: (x) => Promise.resolve(load(x)) }); d.forEach((dep, i) => m.setters[i](load(dep))); m.execute(); return ns; }
load("app");
