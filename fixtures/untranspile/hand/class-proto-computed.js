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
var K = /*#__PURE__*/ function() {
    "use strict";
    function K(v) {
        _class_call_check(this, K);
        this.v = v;
    }
    _create_class(K, [
        {
            key: "hi",
            value: function hi() {
                return "hi " + this.v;
            }
        }
    ], []);
    return K;
}();
var key = ["proto", "type"].join("");
K[key] = {
    hi: function() {
        return "computed";
    }
};
console.log(new K(5).hi());
