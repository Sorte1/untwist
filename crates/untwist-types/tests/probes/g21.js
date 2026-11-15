(function () {
  let x = 1;
  const o = {
    set v(n) {
      x = n;
    },
  };
  o.v = "s";
  console.log(typeof x);
  let y = 1;
  class A {
    static #p = (y = "t");
  }
  console.log(typeof y);
  let z = 0;
  const key = { toString() { z = "side"; return "k"; } };
  ({})[key];
  console.log(typeof z);
  let w = 1;
  function tag() { w = arguments[0].raw; }
  tag`a`;
  console.log(typeof w);
})();
