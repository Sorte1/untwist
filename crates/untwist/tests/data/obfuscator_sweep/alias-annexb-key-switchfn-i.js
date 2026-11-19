function f() {
  var r = [];
  var h = {};
  h.abcde = "x";
  h["fghij"] = function (a, b) { return a + b; };
  var i = h;
  r.push(typeof i, typeof h, i === h, typeof i.abcde, h.abcde);
  switch (r.length) { default: function i() {} }
  r.push(typeof i, typeof h, i === h, typeof i.abcde, h.abcde);
  console.log(r.join());
}
try { f(1); } catch (e) { console.log(e.constructor.name); }
