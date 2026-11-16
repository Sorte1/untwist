(() => { "use strict"; var r = { 1: (e, t, n) => { var a = n(2); console.log("pn", a.q()); },
2: (e, t, n) => { e.exports = { q: function () { return [n(3), new Error().stack.split("\n").length > 1, n.length === undefined].join(" | "); } }; },
3: (e) => { e.exports = 7; } }, c = {}; function o(i) { var a = c[i]; if (void 0 !== a) return a.exports; var m = c[i] = { exports: {} }; return r[i](m, m.exports, o), m.exports; } o.p = "/pub/"; o.c = c; o(1); })();
