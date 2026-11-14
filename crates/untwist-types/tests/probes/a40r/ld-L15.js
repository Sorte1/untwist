(function () {
  document.body.insertAdjacentHTML("beforeend", "<img src=x onerror=\"Math.random = () => null\">");

  const r = Math.random();
  const p = parseInt("12");
  console.log(typeof r, typeof p);
})();
