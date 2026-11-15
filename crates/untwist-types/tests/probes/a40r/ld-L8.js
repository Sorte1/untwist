(function () {
  var w = "wri" + "te"; document[w]("<script>Math.random = () => null</script>");

  const r = Math.random();
  const p = parseInt("12");
  console.log(typeof r, typeof p);
})();
