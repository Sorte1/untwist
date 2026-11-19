const obf_0x4c37ef = function () {
  let _0x972168 = true;
  return function (_0x1940f3, _0x121540) {
    {
      const _0x22bded = _0x972168 ? function () {{if (_0x121540) {const _0x4c8a2d = _0x121540.apply(_0x1940f3, arguments);_0x121540 = null;return _0x4c8a2d;}}} : function () {};
      _0x972168 = false;
      return _0x22bded;
    }
  };
}();
(function () {
  obf_0x4c37ef(this, () => {
    const _0x239326 = new RegExp("function *\\( *\\)");
    const _0x19946e = new RegExp("\\+\\+ *(?:[a-zA-Z_$][0-9a-zA-Z_$]*)", "i");
    const _0x56c26c = obf_0x4fa471("init");
    if (!_0x239326.test(_0x56c26c + "chain") || !_0x19946e.test(_0x56c26c + "input")) {
      _0x56c26c("0");
    } else {
      obf_0x4fa471();
    }
  })();
})();
(function () {
  const _0x610814 = () => {
    {
      let _0xd72b11;
      try {
        _0xd72b11 = Function("return (function() {}.constructor(\"return this\")( ));")();
      } catch (_0x463501) {
        _0xd72b11 = window;
      }
      return _0xd72b11;
    }
  };
  const _0x3e3eb4 = _0x610814();
  _0x3e3eb4.setInterval(obf_0x4fa471, 4e3);
})();
const obf_0x41649b = function () {
  let _0x12dc69 = true;
  return function (_0x254f34, _0x2a041e) {
    {
      const _0x5d3609 = _0x12dc69 ? function () {{if (_0x2a041e) {const _0x34fb08 = _0x2a041e.apply(_0x254f34, arguments);_0x2a041e = null;return _0x34fb08;}}} : function () {};
      _0x12dc69 = false;
      return _0x5d3609;
    }
  };
}();
const obf_0x2816d9 = obf_0x41649b(this, () => {
  const _0xb80d60 = () => {
    {
      let _0x5a4287;
      try {
        _0x5a4287 = Function("return (function() {}.constructor(\"return this\")( ));")();
      } catch (_0x2057ba) {
        _0x5a4287 = window;
      }
      return _0x5a4287;
    }
  };
  const _0x407cff = _0xb80d60();
  const _0x1ccfaf = _0x407cff.console = _0x407cff.console || {};
  const _0x1ed17b = ["log", "warn", "info", "error", "exception", "table", "trace"];
  for (let _0x212782 = 0; _0x212782 < _0x1ed17b.length; _0x212782++) {
    const _0xf10de5 = obf_0x41649b.constructor.prototype.bind(obf_0x41649b);
    const _0x2ababf = _0x1ed17b[_0x212782];
    const _0x4e650a = _0x1ccfaf[_0x2ababf] || _0xf10de5;
    _0xf10de5.__proto__ = obf_0x41649b.bind(obf_0x41649b);
    _0xf10de5.toString = _0x4e650a.toString.bind(_0x4e650a);
    _0x1ccfaf[_0x2ababf] = _0xf10de5;
  }
});
obf_0x2816d9();
class obf_0x4dc745 {
  constructor(_0x4004ee, _0x1374fd) {
    this.name = _0x4004ee;
    this.legs = _0x1374fd;
  }
  describe() {
    return this.name + " walks on " + this.legs + " legs";
  }
  static compare(_0x2df25d, _0x2bed0c) {
    return _0x2df25d.legs - _0x2bed0c.legs;
  }
  get kind() {
    return "animal";
  }
}
class obf_0x4c6a31 extends obf_0x4dc745 {
  constructor(_0xf6e31b) {
    super(_0xf6e31b, 2);
    this.wings = 2;
  }
  describe() {
    return super.describe() + " and flies with " + this.wings + " wings";
  }
  get kind() {
    return "bird";
  }
}
class obf_0x53e138 {
  count = 0;
  add(_0x134f34) {
    this.count += _0x134f34;
    return this;
  }
}
const obf_0x3531bc = [new obf_0x4dc745("cat", 4), new obf_0x4c6a31("crow"), new obf_0x4dc745("snail", 0)];
obf_0x3531bc.sort(obf_0x4dc745.compare);
for (const obf_0x4efc40 of obf_0x3531bc) {
  console.log(obf_0x4efc40.describe(), obf_0x4efc40.kind);
}
console.log("is bird", obf_0x3531bc[1] instanceof obf_0x4c6a31, obf_0x3531bc[1] instanceof obf_0x4dc745);
console.log("tally", new obf_0x53e138().add(3).add(4).count);
function obf_0x4fa471(_0x28f51f) {
  function _0x244ee8(_0x131996) {
    if (typeof _0x131996 === "string") {
      return function (_0x34d8c1) {}.constructor("while (true) {}").apply("counter");
    } else if (("" + _0x131996 / _0x131996).length !== 1 || _0x131996 % 20 === 0) {
      (function () {
        return true;
      }).constructor("debugger").call("action");
    } else {
      (function () {
        return false;
      }).constructor("debugger").apply("stateObject");
    }
    _0x244ee8(++_0x131996);
  }
  try {
    if (_0x28f51f) {
      return _0x244ee8;
    } else {
      _0x244ee8(0);
    }
  } catch (_0xc63a98) {}
}
