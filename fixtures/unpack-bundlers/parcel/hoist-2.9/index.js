var $8LNNk$path = require("path");

function $parcel$export(e, n, v, s) {
  Object.defineProperty(e, n, {get: v, set: s, enumerable: true, configurable: true});
}
function $parcel$interopDefault(a) {
  return a && a.__esModule ? a.default : a;
}
var $parcel$global =
typeof globalThis !== 'undefined'
  ? globalThis
  : typeof self !== 'undefined'
  ? self
  : typeof window !== 'undefined'
  ? window
  : typeof global !== 'undefined'
  ? global
  : {};
var $parcel$modules = {};
var $parcel$inits = {};

var parcelRequire = $parcel$global["parcelRequirecfe7"];
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

  $parcel$global["parcelRequirecfe7"] = parcelRequire;
}
parcelRequire.register("4RSKJ", function(module, exports) {

$parcel$export(module.exports, "say", () => $38b6956c406c18b9$export$91d88f71a134afea);
function $38b6956c406c18b9$export$91d88f71a134afea(line) {
    console.log(line);
}
$38b6956c406c18b9$export$91d88f71a134afea("eval say");

});

parcelRequire.register("6AMnV", function(module, exports) {
module.exports = JSON.parse('{"name":"fixture","items":[1,2,3]}');

});

parcelRequire.register("losIJ", function(module, exports) {
module.exports = Promise.resolve(require("./lazy.85926ab9.js")).then(()=>parcelRequire("aiLzh"));

});


var $4RSKJ = parcelRequire("4RSKJ");

var $4RSKJ = parcelRequire("4RSKJ");
(0, $4RSKJ.say)("eval greet");
function $fc5c1f6a04f977c5$export$2e2bcd8739ae039(name) {
    return "hello " + name;
}
function $fc5c1f6a04f977c5$export$ff41067c50879233(name) {
    return name.toUpperCase() + "!";
}


var $e14939880d0fffa5$exports = {};

$parcel$export($e14939880d0fffa5$exports, "add", () => $e14939880d0fffa5$export$e16d8520af44a096);
$parcel$export($e14939880d0fffa5$exports, "mul", () => $e14939880d0fffa5$export$6e3a27864ab166fe);
$parcel$export($e14939880d0fffa5$exports, "triple", () => $e14939880d0fffa5$export$515818ff8d37c07d);

var $4RSKJ = parcelRequire("4RSKJ");
(0, $4RSKJ.say)("eval math");
const $e14939880d0fffa5$export$e16d8520af44a096 = (a, b)=>a + b;
const $e14939880d0fffa5$export$6e3a27864ab166fe = (a, b)=>a * b;
function $e14939880d0fffa5$export$515818ff8d37c07d(n) {
    return $e14939880d0fffa5$export$6e3a27864ab166fe(n, 3);
}



var $4RSKJ = parcelRequire("4RSKJ");
(0, $4RSKJ.say)("eval live");
let $f26e509df0896a9f$export$e575061ef2a070eb = 0;
function $f26e509df0896a9f$export$bc3c0a588b6c8c13() {
    $f26e509df0896a9f$export$e575061ef2a070eb += 1;
}



var $4RSKJ = parcelRequire("4RSKJ");


(0, $4RSKJ.say)("eval reexport");



var $6AMnV = parcelRequire("6AMnV");
var $c5cc75fd32453494$exports = {};

$parcel$export($c5cc75fd32453494$exports, "join", () => $c5cc75fd32453494$export$f7e2c8231c57a8bd, (v) => $c5cc75fd32453494$export$f7e2c8231c57a8bd = v);
$parcel$export($c5cc75fd32453494$exports, "tag", () => $c5cc75fd32453494$export$2b067c6666111485, (v) => $c5cc75fd32453494$export$2b067c6666111485 = v);
var $c5cc75fd32453494$export$f7e2c8231c57a8bd;
var $c5cc75fd32453494$export$2b067c6666111485;

console.log("eval cjs");
$c5cc75fd32453494$export$f7e2c8231c57a8bd = function(a, b) {
    return $8LNNk$path.posix.join(a, b);
};
$c5cc75fd32453494$export$2b067c6666111485 = "cjs-object";


var $eee8bda3cc99a96e$exports = {};

$parcel$export($eee8bda3cc99a96e$exports, "early", () => $eee8bda3cc99a96e$export$a20a50f149c62f11, (v) => $eee8bda3cc99a96e$export$a20a50f149c62f11 = v);
$parcel$export($eee8bda3cc99a96e$exports, "late", () => $eee8bda3cc99a96e$export$1ee8ee30835eab8b, (v) => $eee8bda3cc99a96e$export$1ee8ee30835eab8b = v);
$parcel$export($eee8bda3cc99a96e$exports, "report", () => $eee8bda3cc99a96e$export$f18d5b0af29128b7, (v) => $eee8bda3cc99a96e$export$f18d5b0af29128b7 = v);
var $eee8bda3cc99a96e$export$a20a50f149c62f11;
var $eee8bda3cc99a96e$export$1ee8ee30835eab8b;
var $eee8bda3cc99a96e$export$f18d5b0af29128b7;
console.log("eval cycle-a start");
$eee8bda3cc99a96e$export$a20a50f149c62f11 = "a-early";
var $d061bbd53be7095c$export$156ac52bf2ea2b93;
var $d061bbd53be7095c$export$35dc45686fc2dbd7;
console.log("eval cycle-b");

$d061bbd53be7095c$export$156ac52bf2ea2b93 = "b saw " + $eee8bda3cc99a96e$export$a20a50f149c62f11 + "," + $eee8bda3cc99a96e$export$1ee8ee30835eab8b;
$d061bbd53be7095c$export$35dc45686fc2dbd7 = function() {
    return "b later " + $eee8bda3cc99a96e$export$1ee8ee30835eab8b;
};


$eee8bda3cc99a96e$export$1ee8ee30835eab8b = "a-late";
$eee8bda3cc99a96e$export$f18d5b0af29128b7 = function() {
    return $d061bbd53be7095c$export$156ac52bf2ea2b93 + " " + $d061bbd53be7095c$export$35dc45686fc2dbd7();
};
console.log("eval cycle-a end");


(0, $4RSKJ.say)("eval index");
(0, $4RSKJ.say)("greet " + (0, $fc5c1f6a04f977c5$export$2e2bcd8739ae039)("ada") + " " + (0, $fc5c1f6a04f977c5$export$ff41067c50879233)("bob"));
(0, $4RSKJ.say)("math " + Object.keys($e14939880d0fffa5$exports).sort().join(",") + " " + $e14939880d0fffa5$exports.add(2, 3));
(0, $4RSKJ.say)("live " + (0, $f26e509df0896a9f$export$e575061ef2a070eb));
(0, $f26e509df0896a9f$export$bc3c0a588b6c8c13)();
(0, $4RSKJ.say)("live " + (0, $f26e509df0896a9f$export$e575061ef2a070eb));
(0, $4RSKJ.say)("reexport " + (0, $fc5c1f6a04f977c5$export$ff41067c50879233)("x") + " " + (0, $e14939880d0fffa5$export$515818ff8d37c07d)(4));
(0, $4RSKJ.say)("json " + (0, (/*@__PURE__*/$parcel$interopDefault($6AMnV))).name + " " + (0, (/*@__PURE__*/$parcel$interopDefault($6AMnV))).items.length);
(0, $4RSKJ.say)("cjs " + (0, (/*@__PURE__*/$parcel$interopDefault($c5cc75fd32453494$exports))).join("a", "b") + " " + (0, (/*@__PURE__*/$parcel$interopDefault($c5cc75fd32453494$exports))).tag);
(0, $4RSKJ.say)("cycle " + (0, (/*@__PURE__*/$parcel$interopDefault($eee8bda3cc99a96e$exports))).report());

Promise.resolve((parcelRequire("6AMnV"))).then((lazy)=>(0, $4RSKJ.say)("json lazy " + (lazy.default || lazy).name));

(parcelRequire("losIJ")).then((lazy)=>(0, $4RSKJ.say)("lazy " + lazy.value + " " + lazy.default));


