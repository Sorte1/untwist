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

  var seq = 0;
  var Counter = /** @class */ (function () {
      function Counter(step) {
          var _this = this;
          this.count = 0;
          this.step = 1;
          this.id = ++seq;
          this.label = "c" + this.id;
          this[_a] = "dynamic";
          this.inc = function () {
              _this.count += _this.step;
              return _this;
          };
          if (step)
              this.step = step;
          Counter.instances++;
      }
      var _a;
      _a = "dyn" + seq;
      Counter.instances = 0;
      Counter.created = Counter.instances + 1;
      return Counter;
  }());
  var c = new Counter(2);
  c.inc().inc();
  console.log(c.count, c.label, c.dyn1, Counter.instances, Counter.created);
  var inc = c.inc;
  inc();
  console.log(c.count, Object.keys(c).join(","));
  var Base = /** @class */ (function () {
      function Base() {
          this.setup();
      }
      Base.prototype.setup = function () { };
      return Base;
  }());
  var Derived = /** @class */ (function (_super) {
      __extends(Derived, _super);
      function Derived() {
          var _this = _super !== null && _super.apply(this, arguments) || this;
          _this.value = 5;
          return _this;
      }
      Derived.prototype.setup = function () {
          this.seen = this.value;
      };
      return Derived;
  }(Base));
  var dd = new Derived();
  console.log(dd.seen, dd.value);
  var Shadow = /** @class */ (function () {
      function Shadow() {
      }
      Object.defineProperty(Shadow.prototype, "x", {
          set: function (v) {
              this.setterRan = true;
          },
          enumerable: false,
          configurable: true
      });
      return Shadow;
  }());
  var Defines = /** @class */ (function (_super) {
      __extends(Defines, _super);
      function Defines() {
          var _this = _super !== null && _super.apply(this, arguments) || this;
          _this.x = 1;
          return _this;
      }
      return Defines;
  }(Shadow));
  var df = new Defines();
  console.log(df.x, df.setterRan === true);

})();
