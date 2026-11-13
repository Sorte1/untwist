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

  var extendStatics = function(d, b) {
    extendStatics = Object.setPrototypeOf ||
        ({ __proto__: [] } instanceof Array && function (d, b) { d.__proto__ = b; }) ||
        function (d, b) { for (var p in b) if (Object.prototype.hasOwnProperty.call(b, p)) d[p] = b[p]; };
    return extendStatics(d, b);
  };

  function __extends(d, b) {
    if (typeof b !== "function" && b !== null)
        throw new TypeError("Class extends value " + String(b) + " is not a constructor or null");
    extendStatics(d, b);
    function __() { this.constructor = d; }
    d.prototype = b === null ? Object.create(b) : (__.prototype = b.prototype, new __());
  }

  typeof SuppressedError === "function" ? SuppressedError : function (error, suppressed, message) {
    var e = new Error(message);
    return e.name = "SuppressedError", e.error = error, e.suppressed = suppressed, e;
  };

  var AppError = /** @class */ (function (_super) {
      __extends(AppError, _super);
      function AppError(message, code) {
          var _this = _super.call(this, message) || this;
          _this.name = "AppError";
          _this.code = code;
          return _this;
      }
      AppError.prototype.describe = function () {
          return this.name + "[" + this.code + "]: " + this.message;
      };
      return AppError;
  }(Error));
  var err = new AppError("bad", 7);
  console.log(err instanceof AppError, err instanceof Error, err.message, err.code);
  console.log(typeof err.describe === "function" ? err.describe() : "no method");
  var Stack = /** @class */ (function (_super) {
      __extends(Stack, _super);
      function Stack() {
          return _super !== null && _super.apply(this, arguments) || this;
      }
      Stack.prototype.peek = function () {
          return this[this.length - 1];
      };
      return Stack;
  }(Array));
  var s = new Stack();
  s.push(1, 2, 3);
  console.log(s.length, s instanceof Stack, typeof s.peek === "function" ? s.peek() : "no method");
  var Registry = /** @class */ (function (_super) {
      __extends(Registry, _super);
      function Registry() {
          return _super !== null && _super.apply(this, arguments) || this;
      }
      Registry.prototype.register = function (k, v) {
          this.set(k, v);
          return this;
      };
      return Registry;
  }(Map));
  var r = new Registry();
  console.log(typeof r.register === "function" ? r.register("a", 1).get("a") : "no method", r instanceof Map);

})();
