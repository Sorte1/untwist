(function () {
  var ce = document.createElement.bind(document); var s = ce("script"); s.text = "Math.random = () => null"; document.head.appendChild(s);

  const r = Math.random();
  const p = parseInt("12");
  console.log(typeof r, typeof p);
})();
