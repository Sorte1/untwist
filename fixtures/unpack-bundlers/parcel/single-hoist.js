var $61WRy$path = require("path");


function $parcel$interopDefault(a) {
  return a && a.__esModule ? a.default : a;
}

function $parcel$export(e, n, v, s) {
  Object.defineProperty(e, n, {get: v, set: s, enumerable: true, configurable: true});
}

      var $parcel$global = globalThis;
    
var $parcel$modules = {};
var $parcel$inits = {};

var parcelRequire = $parcel$global["parcelRequire6b20"];

if (parcelRequire == null) {
  parcelRequire = function(id) {
    if (id in $parcel$modules) {
      return $parcel$modules[id].exports;
    }
    if (id in $parcel$inits) {
      var init = $parcel$inits[id];
      delete $parcel$inits[id];
      var module = {id: id, exports: {}};
      $parcel$modules[id] = module;
      init.call(module.exports, module, module.exports);
      return module.exports;
    }
    var err = new Error("Cannot find module '" + id + "'");
    err.code = 'MODULE_NOT_FOUND';
    throw err;
  };

  parcelRequire.register = function register(id, init) {
    $parcel$inits[id] = init;
  };

  $parcel$global["parcelRequire6b20"] = parcelRequire;
}

var parcelRegister = parcelRequire.register;
parcelRegister("4p8mO", function(module, exports) {
module.exports = JSON.parse("{\"name\":\"fixture\",\"items\":[1,2,3]}");

});

function $89db5cb326c0d620$export$91d88f71a134afea(line) {
    console.log(line);
}
$89db5cb326c0d620$export$91d88f71a134afea("eval say");



(0, $89db5cb326c0d620$export$91d88f71a134afea)("eval greet");
function $e9622134bc81d013$export$2e2bcd8739ae039(name) {
    return "hello " + name;
}
function $e9622134bc81d013$export$ff41067c50879233(name) {
    return name.toUpperCase() + "!";
}


var $45ee23009f73bec1$exports = {};

$parcel$export($45ee23009f73bec1$exports, "add", () => $45ee23009f73bec1$export$e16d8520af44a096);
$parcel$export($45ee23009f73bec1$exports, "mul", () => $45ee23009f73bec1$export$6e3a27864ab166fe);
$parcel$export($45ee23009f73bec1$exports, "triple", () => $45ee23009f73bec1$export$515818ff8d37c07d);

(0, $89db5cb326c0d620$export$91d88f71a134afea)("eval math");
const $45ee23009f73bec1$export$e16d8520af44a096 = (a, b)=>a + b;
const $45ee23009f73bec1$export$6e3a27864ab166fe = (a, b)=>a * b;
function $45ee23009f73bec1$export$515818ff8d37c07d(n) {
    return $45ee23009f73bec1$export$6e3a27864ab166fe(n, 3);
}



(0, $89db5cb326c0d620$export$91d88f71a134afea)("eval live");
let $45e17f23454b8a36$export$e575061ef2a070eb = 0;
function $45e17f23454b8a36$export$bc3c0a588b6c8c13() {
    $45e17f23454b8a36$export$e575061ef2a070eb += 1;
}





(0, $89db5cb326c0d620$export$91d88f71a134afea)("eval reexport");



var $4p8mO = parcelRequire("4p8mO");
var $c0041c8c9f7cd6aa$exports = {};

$parcel$export($c0041c8c9f7cd6aa$exports, "join", () => $c0041c8c9f7cd6aa$export$f7e2c8231c57a8bd, (v) => $c0041c8c9f7cd6aa$export$f7e2c8231c57a8bd = v);
$parcel$export($c0041c8c9f7cd6aa$exports, "tag", () => $c0041c8c9f7cd6aa$export$2b067c6666111485, (v) => $c0041c8c9f7cd6aa$export$2b067c6666111485 = v);
var $c0041c8c9f7cd6aa$export$f7e2c8231c57a8bd;
var $c0041c8c9f7cd6aa$export$2b067c6666111485;

console.log("eval cjs");
$c0041c8c9f7cd6aa$export$f7e2c8231c57a8bd = function(a, b) {
    return $61WRy$path.posix.join(a, b);
};
$c0041c8c9f7cd6aa$export$2b067c6666111485 = "cjs-object";


var $595d87f76035a3e9$exports = {};

$parcel$export($595d87f76035a3e9$exports, "early", () => $595d87f76035a3e9$export$a20a50f149c62f11, (v) => $595d87f76035a3e9$export$a20a50f149c62f11 = v);
$parcel$export($595d87f76035a3e9$exports, "late", () => $595d87f76035a3e9$export$1ee8ee30835eab8b, (v) => $595d87f76035a3e9$export$1ee8ee30835eab8b = v);
$parcel$export($595d87f76035a3e9$exports, "report", () => $595d87f76035a3e9$export$f18d5b0af29128b7, (v) => $595d87f76035a3e9$export$f18d5b0af29128b7 = v);
var $595d87f76035a3e9$export$a20a50f149c62f11;
var $595d87f76035a3e9$export$1ee8ee30835eab8b;
var $595d87f76035a3e9$export$f18d5b0af29128b7;
console.log("eval cycle-a start");
$595d87f76035a3e9$export$a20a50f149c62f11 = "a-early";
var $5af7d3480d1db5e1$export$156ac52bf2ea2b93;
var $5af7d3480d1db5e1$export$35dc45686fc2dbd7;
console.log("eval cycle-b");

$5af7d3480d1db5e1$export$156ac52bf2ea2b93 = "b saw " + $595d87f76035a3e9$export$a20a50f149c62f11 + "," + $595d87f76035a3e9$export$1ee8ee30835eab8b;
$5af7d3480d1db5e1$export$35dc45686fc2dbd7 = function() {
    return "b later " + $595d87f76035a3e9$export$1ee8ee30835eab8b;
};


$595d87f76035a3e9$export$1ee8ee30835eab8b = "a-late";
$595d87f76035a3e9$export$f18d5b0af29128b7 = function() {
    return $5af7d3480d1db5e1$export$156ac52bf2ea2b93 + " " + $5af7d3480d1db5e1$export$35dc45686fc2dbd7();
};
console.log("eval cycle-a end");


(0, $89db5cb326c0d620$export$91d88f71a134afea)("eval index");
(0, $89db5cb326c0d620$export$91d88f71a134afea)("greet " + (0, $e9622134bc81d013$export$2e2bcd8739ae039)("ada") + " " + (0, $e9622134bc81d013$export$ff41067c50879233)("bob"));
(0, $89db5cb326c0d620$export$91d88f71a134afea)("math " + Object.keys($45ee23009f73bec1$exports).sort().join(",") + " " + $45ee23009f73bec1$exports.add(2, 3));
(0, $89db5cb326c0d620$export$91d88f71a134afea)("live " + (0, $45e17f23454b8a36$export$e575061ef2a070eb));
(0, $45e17f23454b8a36$export$bc3c0a588b6c8c13)();
(0, $89db5cb326c0d620$export$91d88f71a134afea)("live " + (0, $45e17f23454b8a36$export$e575061ef2a070eb));
(0, $89db5cb326c0d620$export$91d88f71a134afea)("reexport " + (0, $e9622134bc81d013$export$ff41067c50879233)("x") + " " + (0, $45ee23009f73bec1$export$515818ff8d37c07d)(4));
(0, $89db5cb326c0d620$export$91d88f71a134afea)("json " + (0, (/*@__PURE__*/$parcel$interopDefault($4p8mO))).name + " " + (0, (/*@__PURE__*/$parcel$interopDefault($4p8mO))).items.length);
(0, $89db5cb326c0d620$export$91d88f71a134afea)("cjs " + (0, (/*@__PURE__*/$parcel$interopDefault($c0041c8c9f7cd6aa$exports))).join("a", "b") + " " + (0, (/*@__PURE__*/$parcel$interopDefault($c0041c8c9f7cd6aa$exports))).tag);
(0, $89db5cb326c0d620$export$91d88f71a134afea)("cycle " + (0, (/*@__PURE__*/$parcel$interopDefault($595d87f76035a3e9$exports))).report());

Promise.resolve((parcelRequire("4p8mO"))).then((lazy)=>(0, $89db5cb326c0d620$export$91d88f71a134afea)("json lazy " + (lazy.default || lazy).name));


