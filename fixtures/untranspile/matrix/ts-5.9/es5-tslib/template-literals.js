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


  function __makeTemplateObject(cooked, raw) {
    if (Object.defineProperty) { Object.defineProperty(cooked, "raw", { value: raw }); } else { cooked.raw = raw; }
    return cooked;
  }
  typeof SuppressedError === "function" ? SuppressedError : function (error, suppressed, message) {
    var e = new Error(message);
    return e.name = "SuppressedError", e.error = error, e.suppressed = suppressed, e;
  };

  var name = "world";
  var n = 3;
  console.log("hello ".concat(name), "".concat(n, " + ").concat(n, " = ").concat(n + n), "nested ".concat("inner ".concat(name)));
  console.log("multi\nline", "".concat({ toString: function () { return "obj"; } }), "".concat([1, 2]));
  function tag(strings) {
      var values = [];
      for (var _i = 1; _i < arguments.length; _i++) {
          values[_i - 1] = arguments[_i];
      }
      return strings.raw.join("|") + "#" + strings.join("|") + "#" + values.join(",");
  }
  console.log(tag(templateObject_1 || (templateObject_1 = __makeTemplateObject(["a", "b", "c"], ["a", "b", "c"])), 1, 2), tag(templateObject_2 || (templateObject_2 = __makeTemplateObject(["\n", "\t"], ["\\n", "\\t"])), "x"));
  function same(s) {
      return s;
  }
  var first = function () { return same(templateObject_3 || (templateObject_3 = __makeTemplateObject(["x"], ["x"]))); };
  console.log(first() === first(), Object.isFrozen(first()), String.raw(templateObject_4 || (templateObject_4 = __makeTemplateObject(["A", "\n"], ["\\u{41}", "\\n"])), 1));
  var sym = { valueOf: function () { return 1; }, toString: function () { return "str"; } };
  console.log("".concat(sym), "" + sym);
  var templateObject_1, templateObject_2, templateObject_3, templateObject_4;

})();
