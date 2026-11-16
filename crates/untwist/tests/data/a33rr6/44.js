(function () {
  var t, e;
  function r() { if (t) return e; t = 1; e = { who: new Error("x").stack.split("\n")[1].trim().split(" ")[1] }; return e; }
  console.log(r().who);
})();
