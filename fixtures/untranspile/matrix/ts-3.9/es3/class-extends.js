"use strict";
var __extends = (this && this.__extends) || (function () {
    var extendStatics = function (d, b) {
        extendStatics = Object.setPrototypeOf ||
            ({ __proto__: [] } instanceof Array && function (d, b) { d.__proto__ = b; }) ||
            function (d, b) { for (var p in b) if (b.hasOwnProperty(p)) d[p] = b[p]; };
        return extendStatics(d, b);
    };
    return function (d, b) {
        extendStatics(d, b);
        function __() { this.constructor = d; }
        d.prototype = b === null ? Object.create(b) : (__.prototype = b.prototype, new __());
    };
})();
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
