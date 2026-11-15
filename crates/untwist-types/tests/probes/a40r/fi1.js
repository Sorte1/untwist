(function () {
  for (var x = 1 in { k: 1 }) {}
  const y = x;
  const g = (c) => { console.log(typeof c); return c; };
  const z = g(x);
  console.log(typeof y, typeof z);
})();
