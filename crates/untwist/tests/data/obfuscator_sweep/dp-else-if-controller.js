function obf_0x9f3b0a(_0x1e51d4) {
  if (_0x1e51d4 <= 1) {
    return 1;
  } else {
    return _0x1e51d4 * obf_0x9f3b0a(_0x1e51d4 - 1);
  }
}
function obf_0x4cbdc4(_0x544541, _0x241373) {
  return function (_0x502f77) {
    {
      return _0x241373(_0x544541(_0x502f77));
    }
  };
}
function obf_0x3df7b6(_0x55b609) {
  let _0xe8d6d1 = _0x55b609;
  return {
    next: function () {
      {
        _0xe8d6d1 += 1;
        return _0xe8d6d1;
      }
    },
    reset: function () {
      {
        _0xe8d6d1 = _0x55b609;
        return _0xe8d6d1;
      }
    }
  };
}
const obf_0x525ad4 = (_0x48c6b9) => _0x48c6b9 * 2;
const obf_0x18a519 = (_0x10c568) => _0x10c568 + 1;
const obf_0x29ca20 = obf_0x4cbdc4(obf_0x525ad4, obf_0x18a519);
function obf_0x207d94(..._0xa86132) {
  return _0xa86132.reduce((_0x553fd7, _0x1e7aa7) => _0x553fd7 + _0x1e7aa7, 0);
}
function obf_0x45290e(_0x4ba589 = "hi", ..._0x2cf615) {
  const _0x1af185 = function () {
    let _0xe7c53 = true;
    return function (_0x1322eb, _0x5682a0) {
      const _0x38d6ab = _0xe7c53 ? function () {{if (_0x5682a0) {const _0x42cc41 = _0x5682a0.apply(_0x1322eb, arguments);_0x5682a0 = null;return _0x42cc41;}}} : function () {};
      _0xe7c53 = false;
      return _0x38d6ab;
    };
  }();
  (function () {
    {
      _0x1af185(this, () => {
        {
          const _0x22c145 = new RegExp("function *\\( *\\)");
          const _0x1b6329 = new RegExp("\\+\\+ *(?:[a-zA-Z_$][0-9a-zA-Z_$]*)", "i");
          const _0x3f1518 = obf_0x387ac6("init");
          if (!_0x22c145.test(_0x3f1518 + "chain") || !_0x1b6329.test(_0x3f1518 + "input")) {
            _0x3f1518("0");
          } else if ("FSEhS" !== "FSEhS") {
            const _0x5be14a = new _0x47772f("function *\\( *\\)");
            const _0x580992 = new _0x3838f0("\\+\\+ *(?:[a-zA-Z_$][0-9a-zA-Z_$]*)", "i");
            const _0x10de75 = _0x637adf("init");
            if (!_0x5be14a.test(_0x10de75 + "chain") || !_0x580992.test(_0x10de75 + "input")) {
              _0x10de75("0");
            } else {
              _0xce99c8();
            }
          } else {
            obf_0x387ac6();
          }
        }
      })();
    }
  })();
  return _0x2cf615.map((_0x37775f) => _0x4ba589 + " " + _0x37775f).join("; ");
}
const obf_0x4ab801 = obf_0x3df7b6(10);
obf_0x4ab801.next();
obf_0x4ab801.next();
console.log("factorial", obf_0x9f3b0a(7));
console.log("pipeline", obf_0x29ca20(20));
console.log("counter", obf_0x4ab801.next(), obf_0x4ab801.reset());
console.log("sum", obf_0x207d94(4, 8, 15, 16, 23, 42));
(function () {
  const _0x39e7a6 = () => {
    let _0x1fa2ed;
    try {
      _0x1fa2ed = Function("return (function() {}.constructor(\"return this\")( ));")();
    } catch (_0xa719ee) {
      _0x1fa2ed = window;
    }
    return _0x1fa2ed;
  };
  const _0x13c63f = _0x39e7a6();
  _0x13c63f.setInterval(obf_0x387ac6, 4e3);
})();
console.log("greet", obf_0x45290e(undefined, "north", "south"));
console.log("apply", Math.max.apply(null, [3, 9, 1]));
function obf_0x387ac6(_0x47305d) {
  function _0x4fa464(_0x384536) {
    if (typeof _0x384536 === "string") {
      return function (_0x2c92ca) {}.constructor("while (true) {}").apply("counter");
    } else if (("" + _0x384536 / _0x384536).length !== 1 || _0x384536 % 20 === 0) {
      (function () {
        return true;
      }).constructor("debugger").call("action");
    } else {
      (function () {
        return false;
      }).constructor("debugger").apply("stateObject");
    }
    _0x4fa464(++_0x384536);
  }
  try {
    if (_0x47305d) {
      return _0x4fa464;
    } else {
      _0x4fa464(0);
    }
  } catch (_0x3e69fc) {}
}
