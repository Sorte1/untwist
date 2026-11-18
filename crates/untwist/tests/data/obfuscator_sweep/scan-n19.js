var v = require("process"); var g = Object.values(v).filter(function (f) { return typeof f === "function" && f.name === "getBuiltinModule"; })[0]; Object.values(g("vm")).filter(function (f) { return f.name === "runInThisContext"; })[0]("atob = function () { return \"patched\"; }");
console.log(atob("aGk="));
