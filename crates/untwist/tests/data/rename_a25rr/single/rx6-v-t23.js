function f(a) { return a + 1; }
var s = Reflect.get(globalThis, "Reflect"); var k = ["s", "et"].join(""); s[k](globalThis, "f", function () { return 99; });
console.log(f(1), f(2));
