(function () {
  var t, e, a = [];
  e ??= function () {}; a.push(e.name); e = (function () {}); a.push(e.name); e = async () => {}; a.push(e.name); e = function* () {}; a.push(e.name); e = (class {}); a.push(e.name);
  function r() { if (t) return e; t = 1; e = { a: 1 }; return e; }
  console.log(a.join(), r().a);
})();
