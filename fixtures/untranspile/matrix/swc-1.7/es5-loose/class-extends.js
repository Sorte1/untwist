"use strict";
function _defineProperties(target, props) {
    for(var i = 0; i < props.length; i++){
        var descriptor = props[i];
        descriptor.enumerable = descriptor.enumerable || false;
        descriptor.configurable = true;
        if ("value" in descriptor) descriptor.writable = true;
        Object.defineProperty(target, descriptor.key, descriptor);
    }
}
function _create_class(Constructor, protoProps, staticProps) {
    if (protoProps) _defineProperties(Constructor.prototype, protoProps);
    if (staticProps) _defineProperties(Constructor, staticProps);
    return Constructor;
}
function _inherits(subClass, superClass) {
    if (typeof superClass !== "function" && superClass !== null) {
        throw new TypeError("Super expression must either be null or a function");
    }
    subClass.prototype = Object.create(superClass && superClass.prototype, {
        constructor: {
            value: subClass,
            writable: true,
            configurable: true
        }
    });
    if (superClass) _set_prototype_of(subClass, superClass);
}
function _instanceof(left, right) {
    if (right != null && typeof Symbol !== "undefined" && right[Symbol.hasInstance]) {
        return !!right[Symbol.hasInstance](left);
    } else {
        return left instanceof right;
    }
}
function _set_prototype_of(o, p) {
    _set_prototype_of = Object.setPrototypeOf || function setPrototypeOf(o, p) {
        o.__proto__ = p;
        return o;
    };
    return _set_prototype_of(o, p);
}
var Animal = /*#__PURE__*/ function() {
    "use strict";
    function Animal(name) {
        this.name = name;
    }
    var _proto = Animal.prototype;
    _proto.speak = function speak() {
        return this.name + " makes a sound";
    };
    Animal.create = function create(name) {
        return new this(name);
    };
    _create_class(Animal, [
        {
            key: "kind",
            get: function get() {
                return "animal";
            }
        }
    ]);
    return Animal;
}();
var Dog = /*#__PURE__*/ function(Animal) {
    "use strict";
    _inherits(Dog, Animal);
    function Dog(name, breed) {
        var _this;
        _this = Animal.call(this, name) || this;
        _this.breed = breed;
        return _this;
    }
    var _proto = Dog.prototype;
    _proto.speak = function speak() {
        return Animal.prototype.speak.call(this) + " (woof)";
    };
    Dog.create = function create(name) {
        var d = Animal.create.call(this, name);
        d.breed = "mutt";
        return d;
    };
    _create_class(Dog, [
        {
            key: "kind",
            get: function get() {
                return "dog:" + Animal.prototype.kind;
            }
        }
    ]);
    return Dog;
}(Animal);
var Puppy = /*#__PURE__*/ function(Dog) {
    "use strict";
    _inherits(Puppy, Dog);
    function Puppy() {
        return Dog.apply(this, arguments) || this;
    }
    return Puppy;
}(Dog);
var d = new Dog("Rex", "lab");
console.log(d.speak(), d.kind, _instanceof(d, Animal), d.breed);
var m = Dog.create("Max");
console.log(_instanceof(m, Dog), m.breed, m.speak());
var pup = new Puppy("Bit", "pug");
console.log(pup.speak(), Object.getPrototypeOf(Puppy) === Dog, Puppy.name);
function Legacy(v) {
    this.v = v;
}
Legacy.prototype.get = function() {
    return this.v;
};
var Modern = /*#__PURE__*/ function(Legacy) {
    "use strict";
    _inherits(Modern, Legacy);
    function Modern() {
        return Legacy.apply(this, arguments) || this;
    }
    var _proto = Modern.prototype;
    _proto.get = function get() {
        return Legacy.prototype.get.call(this) * 2;
    };
    return Modern;
}(Legacy);
console.log(new Modern(21).get());
