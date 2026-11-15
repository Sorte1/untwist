console.log(["_0xo"].map(function (n) {
  var t, w;
  try { t = (0, eval)("typeof " + n); } catch (e) { t = e.name; }
  try { (0, eval)(n + " = " + n); w = "w"; } catch (e) { w = e.name; }
  return n + ":" + t + ":" + w + ":" + Object.prototype.hasOwnProperty.call(globalThis, n);
}).join(","));
