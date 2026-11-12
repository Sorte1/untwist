"use strict";
function _assert_this_initialized(self) {
    if (self === void 0) throw new ReferenceError("this hasn't been initialised - super() hasn't been called");
    return self;
}
function _call_super(_this, derived, args) {
    derived = _get_prototype_of(derived);
    return _possible_constructor_return(_this, _is_native_reflect_construct() ? Reflect.construct(derived, args || [], _get_prototype_of(_this).constructor) : derived.apply(_this, args));
}
function _class_call_check(instance, Constructor) {
    if (!(instance instanceof Constructor)) throw new TypeError("Cannot call a class as a function");
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
function _create_class(Constructor, protoProps, staticProps) {
    if (protoProps) _defineProperties(Constructor.prototype, protoProps);
    if (staticProps) _defineProperties(Constructor, staticProps);
    return Constructor;
}
function _get(target, property, receiver) {
    if (typeof Reflect !== "undefined" && Reflect.get) _get = Reflect.get;
    else {
        _get = function get(target, property, receiver) {
            var base = _super_prop_base(target, property);
            if (!base) return;
            var desc = Object.getOwnPropertyDescriptor(base, property);
            if (desc.get) return desc.get.call(receiver || target);
            return desc.value;
        };
    }
    return _get(target, property, receiver || target);
}
function _get_prototype_of(o) {
    _get_prototype_of = Object.setPrototypeOf ? Object.getPrototypeOf : function getPrototypeOf(o) {
        return o.__proto__ || Object.getPrototypeOf(o);
    };
    return _get_prototype_of(o);
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
    "@swc/helpers - instanceof";
    if (right != null && typeof Symbol !== "undefined" && right[Symbol.hasInstance]) {
        return !!right[Symbol.hasInstance](left);
    } else return left instanceof right;
}
function _is_native_reflect_construct() {
    try {
        var result = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function() {}));
    } catch (_) {}
    return (_is_native_reflect_construct = function() {
        return !!result;
    })();
}
function _possible_constructor_return(self, call) {
    if (call && (_type_of(call) === "object" || typeof call === "function")) return call;
    return _assert_this_initialized(self);
}
function _set_prototype_of(o, p) {
    _set_prototype_of = Object.setPrototypeOf || function setPrototypeOf(o, p) {
        o.__proto__ = p;
        return o;
    };
    return _set_prototype_of(o, p);
}
function _super_prop_base(object, property) {
    while(!Object.prototype.hasOwnProperty.call(object, property)){
        object = _get_prototype_of(object);
        if (object === null) break;
    }
    return object;
}
function _type_of(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
}
var Animal = /*#__PURE__*/ function() {
    "use strict";
    function Animal(name) {
        _class_call_check(this, Animal);
        this.name = name;
    }
    _create_class(Animal, [
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
    function Dog(name, breed) {
        _class_call_check(this, Dog);
        var _this;
        _this = _call_super(this, Dog, [
            name
        ]);
        _this.breed = breed;
        return _this;
    }
    _create_class(Dog, [
        {
            key: "speak",
            value: function speak() {
                return _get(_get_prototype_of(Dog.prototype), "speak", this).call(this) + " (woof)";
            }
        },
        {
            key: "kind",
            get: function get() {
                return "dog:" + _get(_get_prototype_of(Dog.prototype), "kind", this);
            }
        }
    ], [
        {
            key: "create",
            value: function create(name) {
                var d = _get(_get_prototype_of(Dog), "create", this).call(this, name);
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
    function Puppy() {
        _class_call_check(this, Puppy);
        return _call_super(this, Puppy, arguments);
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
        _class_call_check(this, Modern);
        return _call_super(this, Modern, arguments);
    }
    _create_class(Modern, [
        {
            key: "get",
            value: function get() {
                return _get(_get_prototype_of(Modern.prototype), "get", this).call(this) * 2;
            }
        }
    ]);
    return Modern;
}(Legacy);
console.log(new Modern(21).get());
