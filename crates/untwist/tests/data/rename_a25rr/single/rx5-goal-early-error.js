function f(a) { return a + 1; }
try { Function("let x; let x; f = function () { return 99; }")(); } catch (e) { console.log(e.constructor.name); }
console.log(f(1), f(2));
