Math.imul = function (a, b) {
  return 1;
};
const seed = Date.now() & 0x7fffc | 1;
console.log((Math.imul(seed | 3, (seed | 3) + 1) & 1) === 0 ? "folded" : "patched");
