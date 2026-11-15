(function () {
  import("data:text/javascript,globalThis.parseInt = () => null; Math.random = () => 'r';").then(function () {
    const p = parseInt("12");
    const r = Math.random();
    console.log(typeof p, typeof r);
  });
})();
