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

  var Animal = /** @class */ (function () {
      function Animal(name) {
          this.name = name;
      }
      Animal.prototype.speak = function () {
          return this.name + " makes a sound";
      };
      Animal.create = function (name) {
          return new this(name);
      };
      Object.defineProperty(Animal.prototype, "kind", {
          get: function () {
              return "animal";
          },
          enumerable: false,
          configurable: true
      });
      return Animal;
  }());
  var Dog = /** @class */ (function (_super) {
      __extends(Dog, _super);
      function Dog(name, breed) {
          var _this = _super.call(this, name) || this;
          _this.breed = breed;
          return _this;
      }
      Dog.prototype.speak = function () {
          return _super.prototype.speak.call(this) + " (woof)";
      };
      Object.defineProperty(Dog.prototype, "kind", {
          get: function () {
              return "dog:" + _super.prototype.kind;
          },
          enumerable: false,
          configurable: true
      });
      Dog.create = function (name) {
          var d = _super.create.call(this, name);
          d.breed = "mutt";
          return d;
      };
      return Dog;
  }(Animal));
  var Puppy = /** @class */ (function (_super) {
      __extends(Puppy, _super);
      function Puppy() {
          return _super !== null && _super.apply(this, arguments) || this;
      }
      return Puppy;
  }(Dog));
  var d = new Dog("Rex", "lab");
  console.log(d.speak(), d.kind, d instanceof Animal, d.breed);
  var m = Dog.create("Max");
  console.log(m instanceof Dog, m.breed, m.speak());
  var pup = new Puppy("Bit", "pug");
  console.log(pup.speak(), Object.getPrototypeOf(Puppy) === Dog, Puppy.name);
  function Legacy(v) {
      this.v = v;
  }
  Legacy.prototype.get = function () {
      return this.v;
  };
  var Modern = /** @class */ (function (_super) {
      __extends(Modern, _super);
      function Modern() {
          return _super !== null && _super.apply(this, arguments) || this;
      }
      Modern.prototype.get = function () {
          return _super.prototype.get.call(this) * 2;
      };
      return Modern;
  }(Legacy));
  console.log(new Modern(21).get());

})();
