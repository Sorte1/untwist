function outer(atob, a) {
  a = function () { return atob("aGk="); };
  return a();
}
console.log(outer(function (s) { return "shadow:" + s; }, 0));
