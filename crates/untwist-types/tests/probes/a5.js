(function () {
  async function f() {
    return 1;
  }
  const p = f();
  p.valueOf = () => 1n;
  const r = p * 2n;
  const m = -p;
  console.log(typeof r, typeof m);
})();
