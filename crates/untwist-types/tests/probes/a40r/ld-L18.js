(function () {
  var f = document.createElement("iframe"); document.body.appendChild(f); f.contentWindow.Function("parent.Math.random = () => null")();

  const r = Math.random();
  const p = parseInt("12");
  console.log(typeof r, typeof p);
})();
