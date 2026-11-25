const seed = Date.now() & 0x7fffc | 1;
function patch(target) {
  target.Math.imul = function (a, b) {
    return Number(BigInt.asIntN(32, BigInt(a | 0) * BigInt(b | 0)));
  };
}
(function () {
  const root = typeof globalThis !== "undefined" ? globalThis : {};
  patch(root);
})();
console.log((Math.imul(seed | 3, (seed | 3) + 1) & 1) === 0 ? "even" : "odd");
