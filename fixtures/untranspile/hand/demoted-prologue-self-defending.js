(function () {
  var c = (function () { var f = !![]; return function (g, h) { var i = f ? function () { if (h) { var j = h['apply'](g, arguments); h = null; return j; } } : function () {}; f = ![]; return i; }; })();
  var s = c(this, function () { return s['toString']()['search']('(((.+)+)+)+$')['toString']()['constructor'](s)['search']('(((.+)+)+)+$'); });
  s();
  'use strict';
  x = 1;
  console.log(typeof x, (function () { return this === undefined; })());
})();
