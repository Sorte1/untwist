(function () {
  function w() { var { caller: c } = w; return c.name; }
  function main() { var _0x1 = { 'abcde': function (a) { return a(); }, 'fghij': function (a, b) { return a + b; } }; return _0x1['abcde'](w) + _0x1['fghij'](1, 2); }
  console.log(main());
})();
