const N = (f) => Error.prototype.toString.call(f);
const r = require(require("path").resolve(process.argv[2]));
console.log("result " + (typeof r === "function" ? JSON.stringify(N(r)) : Object.keys(r).map((k) => k + "=" + JSON.stringify(typeof r[k] === "function" ? N(r[k]) : r[k])).join(",")));
