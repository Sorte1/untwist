"use strict";

var _typeof = typeof Symbol === "function" && typeof Symbol.iterator === "symbol" ? function (obj) { return typeof obj; } : function (obj) { return obj && typeof Symbol === "function" && obj.constructor === Symbol && obj !== Symbol.prototype ? "symbol" : typeof obj; };

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

var _loop2 = function _loop2(i) {
  if (i === 1) return "continue";
  if (i === 3) return "break";
  later.push(function () {
    return i * 10;
  });
};

_loop3: for (var i = 0; i < 4; i++) {
  var _ret2 = _loop2(i);

  switch (_ret2) {
    case "continue":
      continue;

    case "break":
      break _loop3;}
}

console.log(later.map(function (f) {
  return f();
}).join(","));
function find(list) {
  var _loop4 = function _loop4(item) {
    var handler = function handler() {
      return item.id;
    };
    if (item.ok) return {
        v: handler()
      };
  };

  var _iteratorNormalCompletion = true;
  var _didIteratorError = false;
  var _iteratorError = undefined;

  try {
    for (var _iterator = list[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true) {
      var item = _step.value;

      var _ret3 = _loop4(item);

      if ((typeof _ret3 === "undefined" ? "undefined" : _typeof(_ret3)) === "object") return _ret3.v;
    }
  } catch (err) {
    _didIteratorError = true;
    _iteratorError = err;
  } finally {
    try {
      if (!_iteratorNormalCompletion && _iterator.return) {
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
console.log(find([{ id: 1 }, { id: 2, ok: true }]));
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
