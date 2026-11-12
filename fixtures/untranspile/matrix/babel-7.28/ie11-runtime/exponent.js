(function () {
  'use strict';

  var _;
  var x = 2;
  console.log(Math.pow(2, 10), Math.pow(-2, 3), Math.pow(2, -1), Math.pow(2, Math.pow(3, 2)));
  x = Math.pow(x, 3);
  console.log(x);
  var o = {
    v: 3
  };
  o.v = Math.pow(o.v, 2);
  var arr = [4];
  arr[_ = 0] = Math.pow(arr[_], 0.5);
  console.log(o.v, arr[0], Math.pow(10n, 3n) === 1000n);

})();
