(function () {
  function f(n, o) { o = function () {}; n.x = o; return [arguments[1] === o, o.name]; }
  console.log(f({}, 1).join());
  function g(n, o) { o = function () {}; n.x = o; return [g.arguments[1] === o, o.name]; }
  console.log(g({}, 1).join());
  function h(n, o) { o = function () {}; n.x = o; return k(); }
  function k() { return [k.caller.arguments[1] === h.arguments[1], typeof h.arguments[1], h.length]; }
  console.log(h({}, 1).join());
  var m = (n, o) => { o = () => 1; return o.name + m.length; };
  console.log(m({}, 1));
})();
