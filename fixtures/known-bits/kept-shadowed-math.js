const seed = Date.now() & 0x7fffc | 1;
function run(Math) {
  return (Math.imul(seed | 3, (seed | 3) + 1) & 1) === 0 ? "folded" : "shadowed";
}
console.log(run({ imul: function () { return 1; } }));
