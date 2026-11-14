(function () {
  const sc = document.createElement("script"); sc.textContent = "parseInt = () => null"; document.head.appendChild(sc);
  const r = Math.random();
  const p = parseInt("12");
  const n = (5).toFixed(2);
  const u = "abc".trim();
  console.log(typeof r, typeof p, typeof n, typeof u);
})();
