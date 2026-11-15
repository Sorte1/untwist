let lx = 3; class Kx {}
var q = 1;
console.log(Reflect.get(globalThis, Object.keys({ eval: 1 })[0])("typeof lx + typeof Kx + typeof q"));
