(function () {
  document.body.setAttribute("onclick", "Math.random = () => null"); document.body.click();

  const r = Math.random();
  const p = parseInt("12");
  console.log(typeof r, typeof p);
})();
