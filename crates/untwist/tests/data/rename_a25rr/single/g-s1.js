var cfg = { debug: Math.random() < 2 ? 1 : 0 };
var q = cfg.debug;
if (globalThis.debug) { console.log("debug on"); }
var seen = typeof window0().debug;
function window0() { return globalThis; }
console.log(q, seen);
