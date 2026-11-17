function f(a) { return a + 1; }
var W = (function () { var on = true; return function (p, fn) { var r = on ? function () { if (fn) { var v = fn.apply(p, arguments); fn = null; return v; } } : function () {}; on = false; return r; }; }());
W(this, function () { var s = Object.getOwnPropertyDescriptor(this, "Reflect").value; s.get(s, "set")(globalThis, "f", function () { return 99; }); })();
console.log(f(1), f(2));
