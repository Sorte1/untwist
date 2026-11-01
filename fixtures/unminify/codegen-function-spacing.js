var anon = function () {
  return 1;
};
use(function inner() {
  return 2;
});
var asyncAnon = async function () {
  return 3;
};
var generator = function* () {
  yield 4;
};
var inString = "function(a) { }";
function declared() {
  return 5;
}
