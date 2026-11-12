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
function _set_prototype_of(o, p) {
    _set_prototype_of = Object.setPrototypeOf || function setPrototypeOf(o, p) {
        o.__proto__ = p;
        return o;
    };
    return _set_prototype_of(o, p);
}
var seq = 0;
var prop;
var Counter = function Counter(step) {
    "use strict";
    var _this = this;
    this.count = 0;
    this.step = 1;
    this.id = ++seq;
    this.label = "c" + this.id;
    this[prop] = "dynamic";
    this.inc = function() {
        _this.count += _this.step;
        return _this;
    };
    if (step) this.step = step;
    Counter.instances++;
};
prop = "dyn" + seq;
Counter.instances = 0;
Counter.created = Counter.instances + 1;
var c = new Counter(2);
c.inc().inc();
console.log(c.count, c.label, c.dyn1, Counter.instances, Counter.created);
var inc = c.inc;
inc();
console.log(c.count, Object.keys(c).join(","));
var Base = /*#__PURE__*/ function() {
    "use strict";
    function Base() {
        this.setup();
    }
    var _proto = Base.prototype;
    _proto.setup = function setup() {};
    return Base;
}();
var Derived = /*#__PURE__*/ function(Base) {
    "use strict";
    _inherits(Derived, Base);
    function Derived() {
        var _this;
        _this = Base.apply(this, arguments) || this;
        _this.value = 5;
        return _this;
    }
    var _proto = Derived.prototype;
    _proto.setup = function setup() {
        this.seen = this.value;
    };
    return Derived;
}(Base);
var dd = new Derived();
console.log(dd.seen, dd.value);
var Shadow = /*#__PURE__*/ function() {
    "use strict";
    function Shadow() {}
    _create_class(Shadow, [
        {
            key: "x",
            set: function set(v) {
                this.setterRan = true;
            }
        }
    ]);
    return Shadow;
}();
var Defines = /*#__PURE__*/ function(Shadow) {
    "use strict";
    _inherits(Defines, Shadow);
    function Defines() {
        var _this;
        _this = Shadow.apply(this, arguments) || this;
        _this.x = 1;
        return _this;
    }
    return Defines;
}(Shadow);
var df = new Defines();
console.log(df.x, df.setterRan === true);
