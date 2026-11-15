function parseFloat() { return "s"; }
function isNaN() { return 1; }
const JSON = { stringify() { return 5; } };
(function () {
  const p = parseFloat("1.5");
  const q = isNaN(p);
  const j = JSON.stringify({});
  console.log(typeof p, typeof q, typeof j);
})();
