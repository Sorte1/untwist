let lx = 3; class Kx {}
var q = 1;
console.log(Object.entries(globalThis).length, Object.getOwnPropertyNames(globalThis).filter(function (n) { return n.length == 4 && n[0] == "e" && n[1] == "v"; }).map(function (n) { return globalThis[n]; })[0]("typeof lx + typeof Kx + typeof q"));
