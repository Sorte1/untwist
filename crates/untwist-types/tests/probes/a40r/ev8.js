(function () {
  function f(s) {
    eval(s);
    return () => {
      let u = undefined;
      const n = -NaN;
      const g = (c) => { if (c) { return undefined; } return 1; };
      const w = g(false);
      console.log(typeof u, typeof n, typeof g(true), typeof w);
      u = 1;
      return u;
    };
  }
  f("var undefined = 'str', NaN = 5n")();
})();
