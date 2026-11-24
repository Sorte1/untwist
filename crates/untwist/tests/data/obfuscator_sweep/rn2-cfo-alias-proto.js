(function () {
  const cf = { 'abcde': function (a, b) { return a + b; }, 'fghij': 'hello', 'klmno': function (f, a) { return f(a); } };
  const __proto__ = cf;
  const t = {__proto__};
  console.log(Object.keys(t).join(), __proto__['abcde'](1, 2));
})();
