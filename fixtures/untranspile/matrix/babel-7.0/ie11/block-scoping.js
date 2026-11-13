"use strict";

function _typeof(obj) { if (typeof Symbol === "function" && typeof Symbol.iterator === "symbol") { _typeof = function _typeof(obj) { return typeof obj; }; } else { _typeof = function _typeof(obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; }; } return _typeof(obj); }

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
  var _iteratorNormalCompletion = true;
  var _didIteratorError = false;
  var _iteratorError = undefined;

  try {
    var _loop4 = function _loop4() {
      var item = _step.value;

      var handler = function handler() {
        return item.id;
      };

      if (item.ok) return {
        v: handler()
      };
    };

    for (var _iterator = list[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true) {
      var _ret2 = _loop4();

      if (_typeof(_ret2) === "object") return _ret2.v;
    }
  } catch (err) {
    _didIteratorError = true;
    _iteratorError = err;
  } finally {
    try {
      if (!_iteratorNormalCompletion && _iterator.return != null) {
        _iterator.return();
      }
    } finally {
      if (_didIteratorError) {
        throw _iteratorError;
      }
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

var _loop5 = function _loop5() {
  var k = j;
  ws.push(function () {
    return k;
  });
  j++;
};

while (j < 3) {
  _loop5();
}

console.log(ws.map(function (f) {
  return f();
}).join(","));

var _loop6 = function _loop6(n, _i) {
  setTimeout(function () {
    return console.log("timer", _i);
  }, 0);
};

for (var _i = 0, n = 2; _i < n; _i++) {
  _loop6(n, _i);
}

switch (1) {
  case 1:
    {
      var s = "case";
      console.log(s);
    }
}
