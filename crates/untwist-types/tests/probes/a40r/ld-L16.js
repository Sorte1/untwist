(function () {
  var d = document; var s = d.createElement("script"); s.text = "Math.random = () => null"; d.head.appendChild(s);

  const r = Math.random();
  const p = parseInt("12");
  console.log(typeof r, typeof p);
})();
