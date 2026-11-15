(function () {
  function w() { return arguments.callee.caller.name + "/" + (arguments.callee.caller === main); }
  function main() { var _0x1 = { 'abcde': function (a, b) { return a(b); }, 'fghij': function (a, b) { return a === b; } }; return _0x1['abcde'](w, 1) + _0x1['fghij'](1, 2); }
  console.log(main());
})();
