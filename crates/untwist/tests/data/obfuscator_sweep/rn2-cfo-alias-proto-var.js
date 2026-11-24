(function () {
  var cf = { 'abcde': function (a, b) { return a + b; }, 'fghij': 'hello', 'klmno': function (f, a) { return f(a); } };
  var __proto__ = cf;
  var { __proto__: u } = { __proto__ };
  console.log(u === cf, __proto__['abcde'](1, 2));
})();
