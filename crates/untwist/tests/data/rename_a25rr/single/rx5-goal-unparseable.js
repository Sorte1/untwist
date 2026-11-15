function f(a) { return a + 1; }
try { eval("f = function () { return 99; } )"); } catch (e) { console.log(e.constructor.name); }
console.log(f(1), f(2));
