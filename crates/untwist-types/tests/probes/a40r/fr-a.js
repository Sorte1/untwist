(function () {
  function f(a) { console.log(typeof a, a); helper(); return a; }
  const r = f(1);
  console.log(typeof r);
})();
