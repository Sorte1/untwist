"use strict";
function _tagged_template_literal_loose(strings, raw) {
    if (!raw) {
        raw = strings.slice(0);
    }
    strings.raw = raw;
    return strings;
}
function _templateObject() {
    var data = _tagged_template_literal_loose([
        "a",
        "b",
        "c"
    ]);
    _templateObject = function _templateObject() {
        return data;
    };
    return data;
}
function _templateObject1() {
    var data = _tagged_template_literal_loose([
        "\n",
        "	"
    ], [
        "\\n",
        "\\t"
    ]);
    _templateObject1 = function _templateObject() {
        return data;
    };
    return data;
}
function _templateObject2() {
    var data = _tagged_template_literal_loose([
        "x"
    ]);
    _templateObject2 = function _templateObject() {
        return data;
    };
    return data;
}
function _templateObject3() {
    var data = _tagged_template_literal_loose([
        "A",
        "\n"
    ], [
        "\\u{41}",
        "\\n"
    ]);
    _templateObject3 = function _templateObject() {
        return data;
    };
    return data;
}
var name = "world";
var n = 3;
console.log("hello " + name, n + " + " + n + " = " + (n + n), "nested " + ("inner " + name));
console.log("multi\nline", "" + {
    toString: function toString() {
        return "obj";
    }
}, "" + [
    1,
    2
]);
function tag(strings) {
    for(var _len = arguments.length, values = new Array(_len > 1 ? _len - 1 : 0), _key = 1; _key < _len; _key++){
        values[_key - 1] = arguments[_key];
    }
    return strings.raw.join("|") + "#" + strings.join("|") + "#" + values.join(",");
}
console.log(tag(_templateObject(), 1, 2), tag(_templateObject1(), "x"));
function same(s) {
    return s;
}
var first = function() {
    return same(_templateObject2());
};
console.log(first() === first(), Object.isFrozen(first()), String.raw(_templateObject3(), 1));
var sym = {
    valueOf: function valueOf() {
        return 1;
    },
    toString: function toString() {
        return "str";
    }
};
console.log("" + sym, "" + sym);
