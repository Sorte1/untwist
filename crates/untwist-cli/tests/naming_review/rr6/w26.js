(() => { "use strict"; var r = { 2: (e, t, n) => { e.exports = { q: function () { return n(3); } }; },
3: (e) => { e.exports = 7; } }, c = {}; function o(i) { var a = c[i]; if (void 0 !== a) return a.exports; var m = c[i] = { exports: {} }; return r[i](m, m.exports, o), m.exports; } (() => { var a = o(2); Promise.resolve().then(o.bind(o, 3)).then((v) => console.log("pn", a.q(), v)); })(); })();
