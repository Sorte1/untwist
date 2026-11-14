(function () {
  var s = document.createElementNS("http://www.w3.org/1999/xhtml", "script"); s.text = "Math.random = () => 1n"; document.head.appendChild(s);

  const r = Math.random();
  const p = parseInt("12");
  console.log(typeof r, typeof p);
})();
