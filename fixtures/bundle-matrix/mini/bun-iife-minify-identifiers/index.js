(() => {
  var z = Object.create;
  var m = Object.getPrototypeOf;
  var r = Object.defineProperty;
  var p = Object.getOwnPropertyNames;
  var s = Object.prototype.hasOwnProperty;
  function f(o) {
    return this[o];
  }
  var w;
  var x;
  var d = (o, l, e) => {
    var n = o != null && typeof o === "object";
    if (n) {
      var t = l ? w ??= new WeakMap : x ??= new WeakMap;
      var i = t.get(o);
      if (i)
        return i;
    }
    e = o != null ? z(m(o)) : {};
    const a = l || !o || !o.__esModule || !s.call(o, "default") ? r(e, "default", { value: o, enumerable: true }) : e;
    if (o && typeof o === "object" || typeof o === "function") {
      for (let c of p(o))
        if (!s.call(a, c))
          r(a, c, {
            get: f.bind(o, c),
            enumerable: true
          });
    }
    if (n)
      t.set(o, a);
    return a;
  };
  var h = (o, l) => () => (l || o((l = { exports: {} }).exports, l), l.exports);
  var j = (o, l, e) => () => {
    if (o)
      try {
        l = o(o = 0);
      } catch (n) {
        e = [n];
      }
    if (e)
      throw e[0];
    return l;
  };

  // cjs.js
  var g = h(function(v) {
    console.log("eval cjs");
    v.twice = function(o) {
      return o * 2;
    };
  });

  // lazy.js
  var b = 42;
  var u = j(() => {
    console.log("eval lazy");
  });

  // index.js
  var y = d(g());
  console.log("eval index " + y.twice(21));
  Promise.resolve().then(() => (u(), {})).then((o) => console.log("lazy " + b));
})();
