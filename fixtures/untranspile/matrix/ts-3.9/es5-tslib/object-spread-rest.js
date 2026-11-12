(function () {
  'use strict';

  /******************************************************************************
  Copyright (c) Microsoft Corporation.

  Permission to use, copy, modify, and/or distribute this software for any
  purpose with or without fee is hereby granted.

  THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
  REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
  AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
  INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
  LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
  OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
  PERFORMANCE OF THIS SOFTWARE.
  ***************************************************************************** */
  /* global Reflect, Promise, SuppressedError, Symbol, Iterator */


  var __assign = function() {
    __assign = Object.assign || function __assign(t) {
        for (var s, i = 1, n = arguments.length; i < n; i++) {
            s = arguments[i];
            for (var p in s) if (Object.prototype.hasOwnProperty.call(s, p)) t[p] = s[p];
        }
        return t;
    };
    return __assign.apply(this, arguments);
  };

  function __rest(s, e) {
    var t = {};
    for (var p in s) if (Object.prototype.hasOwnProperty.call(s, p) && e.indexOf(p) < 0)
        t[p] = s[p];
    if (s != null && typeof Object.getOwnPropertySymbols === "function")
        for (var i = 0, p = Object.getOwnPropertySymbols(s); i < p.length; i++) {
            if (e.indexOf(p[i]) < 0 && Object.prototype.propertyIsEnumerable.call(s, p[i]))
                t[p[i]] = s[p[i]];
        }
    return t;
  }

  typeof SuppressedError === "function" ? SuppressedError : function (error, suppressed, message) {
    var e = new Error(message);
    return e.name = "SuppressedError", e.error = error, e.suppressed = suppressed, e;
  };

  var _a;
  var base = { a: 1, b: 2 };
  var extra = { c: 3 };
  var merged = __assign(__assign(__assign({}, base), extra), { d: 4 });
  var overridden = __assign(__assign({}, base), { a: 9 });
  console.log(JSON.stringify(merged), JSON.stringify(overridden));
  var a = merged.a, others = __rest(merged, ["a"]);
  console.log(a, JSON.stringify(others));
  function without(_a) {
      _a.secret; var pub = __rest(_a, ["secret"]);
      return pub;
  }
  console.log(JSON.stringify(without({ secret: 1, open: 2 })));
  var withGetter = __assign({ get g() { return "got"; } });
  console.log(Object.getOwnPropertyDescriptor(withGetter, "g").value);
  var sym = Symbol("s");
  var symObj = __assign((_a = {}, _a[sym] = 1, _a.plain = 2, _a));
  console.log(symObj[sym], symObj.plain);
  var _b = symObj, _c = sym, picked = _b[_c], restSym = __rest(_b, [typeof _c === "symbol" ? _c : _c + ""]);
  console.log(picked, Object.keys(restSym).join(","));
  console.log(JSON.stringify(__assign(__assign(__assign(__assign({}, null), undefined), "hi"), [5])));
  var order = [];
  var tracked = { get x() { order.push("x"); return 1; }, get y() { order.push("y"); return 2; } };
  var copy = __assign({}, tracked);
  console.log(order.join(","), copy.x + copy.y);

})();
