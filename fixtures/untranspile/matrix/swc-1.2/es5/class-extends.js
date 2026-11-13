"use strict";
function _assertThisInitialized(self) {
    if (self === void 0) {
        throw new ReferenceError("this hasn't been initialised - super() hasn't been called");
    }
    return self;
}
function _classCallCheck(instance, Constructor) {
    if (!(instance instanceof Constructor)) {
        throw new TypeError("Cannot call a class as a function");
    }
}
function _defineProperties(target, props) {
    for(var i = 0; i < props.length; i++){
        var descriptor = props[i];
        descriptor.enumerable = descriptor.enumerable || false;
        descriptor.configurable = true;
        if ("value" in descriptor) descriptor.writable = true;
        Object.defineProperty(target, descriptor.key, descriptor);
    }
}
function _createClass(Constructor, protoProps, staticProps) {
    if (protoProps) _defineProperties(Constructor.prototype, protoProps);
    if (staticProps) _defineProperties(Constructor, staticProps);
    return Constructor;
}
function _get(target, property, receiver) {
    if (typeof Reflect !== "undefined" && Reflect.get) {
        _get = Reflect.get;
    } else {
        _get = function _get(target, property, receiver) {
            var base = _superPropBase(target, property);
            if (!base) return;
            var desc = Object.getOwnPropertyDescriptor(base, property);
            if (desc.get) {
                return desc.get.call(receiver);
            }
            return desc.value;
        };
    }
    return _get(target, property, receiver || target);
}
function _getPrototypeOf(o) {
    _getPrototypeOf = Object.setPrototypeOf ? Object.getPrototypeOf : function _getPrototypeOf(o) {
        return o.__proto__ || Object.getPrototypeOf(o);
    };
    return _getPrototypeOf(o);
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
    if (superClass) _setPrototypeOf(subClass, superClass);
}
function _instanceof(left, right) {
    if (right != null && typeof Symbol !== "undefined" && right[Symbol.hasInstance]) {
        return !!right[Symbol.hasInstance](left);
    } else {
        return left instanceof right;
    }
}
function _possibleConstructorReturn(self, call) {
    if (call && (_typeof(call) === "object" || typeof call === "function")) {
        return call;
    }
    return _assertThisInitialized(self);
}
function _setPrototypeOf(o, p) {
    _setPrototypeOf = Object.setPrototypeOf || function _setPrototypeOf(o, p) {
        o.__proto__ = p;
        return o;
    };
    return _setPrototypeOf(o, p);
}
function _superPropBase(object, property) {
    while(!Object.prototype.hasOwnProperty.call(object, property)){
        object = _getPrototypeOf(object);
        if (object === null) break;
    }
    return object;
}
var _typeof = function(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
};
function _isNativeReflectConstruct() {
    if (typeof Reflect === "undefined" || !Reflect.construct) return false;
    if (Reflect.construct.sham) return false;
    if (typeof Proxy === "function") return true;
    try {
        Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function() {}));
        return true;
    } catch (e) {
        return false;
    }
}
function _createSuper(Derived) {
    var hasNativeReflectConstruct = _isNativeReflectConstruct();
    return function _createSuperInternal() {
        var Super = _getPrototypeOf(Derived), result;
        if (hasNativeReflectConstruct) {
            var NewTarget = _getPrototypeOf(this).constructor;
            result = Reflect.construct(Super, arguments, NewTarget);
        } else {
            result = Super.apply(this, arguments);
        }
        return _possibleConstructorReturn(this, result);
    };
}
var Animal = /*#__PURE__*/ function() {
    "use strict";
    function Animal(name) {
        _classCallCheck(this, Animal);
        this.name = name;
    }
    _createClass(Animal, [
        {
            key: "speak",
            value: function speak() {
                return this.name + " makes a sound";
            }
        },
        {
            key: "kind",
            get: function get() {
                return "animal";
            }
        }
    ], [
        {
            key: "create",
            value: function create(name) {
                return new this(name);
            }
        }
    ]);
    return Animal;
}();
var Dog = /*#__PURE__*/ function(Animal) {
    "use strict";
    _inherits(Dog, Animal);
    var _super = _createSuper(Dog);
    function Dog(name, breed) {
        _classCallCheck(this, Dog);
        var _this;
        _this = _super.call(this, name);
        _this.breed = breed;
        return _this;
    }
    _createClass(Dog, [
        {
            key: "speak",
            value: function speak() {
                return _get(_getPrototypeOf(Dog.prototype), "speak", this).call(this) + " (woof)";
            }
        },
        {
            key: "kind",
            get: function get() {
                return "dog:" + _get(_getPrototypeOf(Dog.prototype), "kind", this);
            }
        }
    ], [
        {
            key: "create",
            value: function create(name) {
                var d = _get(_getPrototypeOf(Dog), "create", this).call(this, name);
                d.breed = "mutt";
                return d;
            }
        }
    ]);
    return Dog;
}(Animal);
var Puppy = /*#__PURE__*/ function(Dog) {
    "use strict";
    _inherits(Puppy, Dog);
    var _super = _createSuper(Puppy);
    function Puppy() {
        _classCallCheck(this, Puppy);
        return _super.apply(this, arguments);
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
    var _super = _createSuper(Modern);
    function Modern() {
        _classCallCheck(this, Modern);
        return _super.apply(this, arguments);
    }
    _createClass(Modern, [
        {
            key: "get",
            value: function get() {
                return _get(_getPrototypeOf(Modern.prototype), "get", this).call(this) * 2;
            }
        }
    ]);
    return Modern;
}(Legacy);
console.log(new Modern(21).get());
