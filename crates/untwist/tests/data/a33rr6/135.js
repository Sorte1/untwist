(function () {
  function w() { return w.caller.name; }
  var v = { valueOf: function () { return w(); } };
  function main() { var _0x1 = { 'abcde': function (a) { return a(); }, 'fghij': function (a, b) { return a + b; } }; return _0x1['fghij'](v, "") ; }
  console.log(main());
})();
