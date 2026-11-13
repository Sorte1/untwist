"use strict";

var fns = [];

var _loop = function _loop(i) {
  fns.push(function () {
    return i;
  });
};

for (var i = 0; i < 3; i++) {
  _loop(i);
}

console.log(fns.map(function (f) {
  return f();
}).join(","));
var later = [];

var _loop3 = function _loop3(i) {
  if (i === 1) return "continue";
  if (i === 3) return "break";
  later.push(function () {
    return i * 10;
  });
};

_loop2: for (var i = 0; i < 4; i++) {
  var _ret = _loop3(i);

  switch (_ret) {
    case "continue":
      continue;

    case "break":
      break _loop2;
  }
}

console.log(later.map(function (f) {
  return f();
}).join(","));

function find(list) {
  var _loop5 = function _loop5() {
    if (_isArray) {
      if (_i >= _iterator.length) return "break";
      _ref = _iterator[_i++];
    } else {
      _i = _iterator.next();
      if (_i.done) return "break";
      _ref = _i.value;
    }

    var item = _ref;

    var handler = function handler() {
      return item.id;
    };

    if (item.ok) return {
      v: handler()
    };
  };

  _loop4: for (var _iterator = list, _isArray = Array.isArray(_iterator), _i = 0, _iterator = _isArray ? _iterator : _iterator[Symbol.iterator]();;) {
    var _ref;

    var _ret2 = _loop5();

    switch (_ret2) {
      case "break":
        break _loop4;

      default:
        if (typeof _ret2 === "object") return _ret2.v;
    }
  }

  return -1;
}

console.log(find([{
  id: 1
}, {
  id: 2,
  ok: true
}]));
var x = "outer";
{
  var _x = "inner";
  var y = _x + "!";
  console.log(_x, y);
}
console.log(x);
var ws = [];
var j = 0;

var _loop6 = function _loop6() {
  var k = j;
  ws.push(function () {
    return k;
  });
  j++;
};

while (j < 3) {
  _loop6();
}

console.log(ws.map(function (f) {
  return f();
}).join(","));

var _loop7 = function _loop7(n, _i2) {
  setTimeout(function () {
    return console.log("timer", _i2);
  }, 0);
};

for (var _i2 = 0, n = 2; _i2 < n; _i2++) {
  _loop7(n, _i2);
}

switch (1) {
  case 1:
    {
      var s = "case";
      console.log(s);
    }
}
