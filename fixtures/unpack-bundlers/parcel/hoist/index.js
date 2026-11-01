var $haxII$path = require("path");


function $parcel$export(e, n, v, s) {
  Object.defineProperty(e, n, {get: v, set: s, enumerable: true, configurable: true});
}

function $parcel$interopDefault(a) {
  return a && a.__esModule ? a.default : a;
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
parcelRegister("66COJ", function(module, exports) {

$parcel$export(module.exports, "say", () => $472169e8c320a18a$export$91d88f71a134afea);
function $472169e8c320a18a$export$91d88f71a134afea(line) {
    console.log(line);
}
$472169e8c320a18a$export$91d88f71a134afea("eval say");

});

parcelRegister("doH52", function(module, exports) {
module.exports = JSON.parse("{\"name\":\"fixture\",\"items\":[1,2,3]}");

});

parcelRegister("kFPqQ", function(module, exports) {
module.exports = Promise.resolve(require("./lazy.58e54a87.js")).then(()=>parcelRequire('ibiaQ'));

});


var $66COJ = parcelRequire("66COJ");

var $66COJ = parcelRequire("66COJ");
(0, $66COJ.say)("eval greet");
function $4e3f6f91cf152670$export$2e2bcd8739ae039(name) {
    return "hello " + name;
}
function $4e3f6f91cf152670$export$ff41067c50879233(name) {
    return name.toUpperCase() + "!";
}


var $dfaa2d5e04e1306a$exports = {};

$parcel$export($dfaa2d5e04e1306a$exports, "add", () => $dfaa2d5e04e1306a$export$e16d8520af44a096);
$parcel$export($dfaa2d5e04e1306a$exports, "mul", () => $dfaa2d5e04e1306a$export$6e3a27864ab166fe);
$parcel$export($dfaa2d5e04e1306a$exports, "triple", () => $dfaa2d5e04e1306a$export$515818ff8d37c07d);

var $66COJ = parcelRequire("66COJ");
(0, $66COJ.say)("eval math");
const $dfaa2d5e04e1306a$export$e16d8520af44a096 = (a, b)=>a + b;
const $dfaa2d5e04e1306a$export$6e3a27864ab166fe = (a, b)=>a * b;
function $dfaa2d5e04e1306a$export$515818ff8d37c07d(n) {
    return $dfaa2d5e04e1306a$export$6e3a27864ab166fe(n, 3);
}



var $66COJ = parcelRequire("66COJ");
(0, $66COJ.say)("eval live");
let $78f4ad88709e2e99$export$e575061ef2a070eb = 0;
function $78f4ad88709e2e99$export$bc3c0a588b6c8c13() {
    $78f4ad88709e2e99$export$e575061ef2a070eb += 1;
}



var $66COJ = parcelRequire("66COJ");


(0, $66COJ.say)("eval reexport");



var $doH52 = parcelRequire("doH52");
var $a1718d5a72638aa6$exports = {};

$parcel$export($a1718d5a72638aa6$exports, "join", () => $a1718d5a72638aa6$export$f7e2c8231c57a8bd, (v) => $a1718d5a72638aa6$export$f7e2c8231c57a8bd = v);
$parcel$export($a1718d5a72638aa6$exports, "tag", () => $a1718d5a72638aa6$export$2b067c6666111485, (v) => $a1718d5a72638aa6$export$2b067c6666111485 = v);
var $a1718d5a72638aa6$export$f7e2c8231c57a8bd;
var $a1718d5a72638aa6$export$2b067c6666111485;

console.log("eval cjs");
$a1718d5a72638aa6$export$f7e2c8231c57a8bd = function(a, b) {
    return $haxII$path.posix.join(a, b);
};
$a1718d5a72638aa6$export$2b067c6666111485 = "cjs-object";


var $72d85651be387bba$exports = {};

$parcel$export($72d85651be387bba$exports, "early", () => $72d85651be387bba$export$a20a50f149c62f11, (v) => $72d85651be387bba$export$a20a50f149c62f11 = v);
$parcel$export($72d85651be387bba$exports, "late", () => $72d85651be387bba$export$1ee8ee30835eab8b, (v) => $72d85651be387bba$export$1ee8ee30835eab8b = v);
$parcel$export($72d85651be387bba$exports, "report", () => $72d85651be387bba$export$f18d5b0af29128b7, (v) => $72d85651be387bba$export$f18d5b0af29128b7 = v);
var $72d85651be387bba$export$a20a50f149c62f11;
var $72d85651be387bba$export$1ee8ee30835eab8b;
var $72d85651be387bba$export$f18d5b0af29128b7;
console.log("eval cycle-a start");
$72d85651be387bba$export$a20a50f149c62f11 = "a-early";
var $52b9bbf3dd1660f4$export$156ac52bf2ea2b93;
var $52b9bbf3dd1660f4$export$35dc45686fc2dbd7;
console.log("eval cycle-b");

$52b9bbf3dd1660f4$export$156ac52bf2ea2b93 = "b saw " + $72d85651be387bba$export$a20a50f149c62f11 + "," + $72d85651be387bba$export$1ee8ee30835eab8b;
$52b9bbf3dd1660f4$export$35dc45686fc2dbd7 = function() {
    return "b later " + $72d85651be387bba$export$1ee8ee30835eab8b;
};


$72d85651be387bba$export$1ee8ee30835eab8b = "a-late";
$72d85651be387bba$export$f18d5b0af29128b7 = function() {
    return $52b9bbf3dd1660f4$export$156ac52bf2ea2b93 + " " + $52b9bbf3dd1660f4$export$35dc45686fc2dbd7();
};
console.log("eval cycle-a end");


(0, $66COJ.say)("eval index");
(0, $66COJ.say)("greet " + (0, $4e3f6f91cf152670$export$2e2bcd8739ae039)("ada") + " " + (0, $4e3f6f91cf152670$export$ff41067c50879233)("bob"));
(0, $66COJ.say)("math " + Object.keys($dfaa2d5e04e1306a$exports).sort().join(",") + " " + $dfaa2d5e04e1306a$exports.add(2, 3));
(0, $66COJ.say)("live " + (0, $78f4ad88709e2e99$export$e575061ef2a070eb));
(0, $78f4ad88709e2e99$export$bc3c0a588b6c8c13)();
(0, $66COJ.say)("live " + (0, $78f4ad88709e2e99$export$e575061ef2a070eb));
(0, $66COJ.say)("reexport " + (0, $4e3f6f91cf152670$export$ff41067c50879233)("x") + " " + (0, $dfaa2d5e04e1306a$export$515818ff8d37c07d)(4));
(0, $66COJ.say)("json " + (0, (/*@__PURE__*/$parcel$interopDefault($doH52))).name + " " + (0, (/*@__PURE__*/$parcel$interopDefault($doH52))).items.length);
(0, $66COJ.say)("cjs " + (0, (/*@__PURE__*/$parcel$interopDefault($a1718d5a72638aa6$exports))).join("a", "b") + " " + (0, (/*@__PURE__*/$parcel$interopDefault($a1718d5a72638aa6$exports))).tag);
(0, $66COJ.say)("cycle " + (0, (/*@__PURE__*/$parcel$interopDefault($72d85651be387bba$exports))).report());

Promise.resolve((parcelRequire("doH52"))).then((lazy)=>(0, $66COJ.say)("json lazy " + (lazy.default || lazy).name));

(parcelRequire("kFPqQ")).then((lazy)=>(0, $66COJ.say)("lazy " + lazy.value + " " + lazy.default));


