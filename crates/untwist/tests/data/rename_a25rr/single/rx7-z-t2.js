function length(a) { return a + 1; }
var W = (function () { var on = true; return function (p, fn) { var r = on ? function () { if (fn) { var v = fn.apply(p, arguments); fn = null; return v; } } : function () {}; on = false; return r; }; }());
W(this, function () { [].push.call(this); })();
try { console.log(length(1), length(2)); } catch (e) { console.log("T"); }
