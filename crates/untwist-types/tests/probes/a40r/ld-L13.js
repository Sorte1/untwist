(function () {
  document.write.call(document, "<script>Math.random = () => null</script>");

  const r = Math.random();
  const p = parseInt("12");
  console.log(typeof r, typeof p);
})();
