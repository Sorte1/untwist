const seed = Date.now() & 0x7fffc | 1;
(function () {
  const root = typeof globalThis !== "undefined" ? globalThis : {};
  console.log(typeof root.process);
})();
console.log((Math.imul(seed | 3, (seed | 3) + 1) & 1) === 0 ? "even" : "odd");
