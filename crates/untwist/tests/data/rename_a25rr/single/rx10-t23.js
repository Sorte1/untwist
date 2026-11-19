function f(a) { return a + 1; }
var i = 0; this.i = "Reflect"; var [{ set: s }] = [this[i]]; s(this, "f", function () { return 99; });
console.log(f(1), f(2));
