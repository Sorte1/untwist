var $ext$path = require("path");

function $parcel$export(e, n, v, s) {
  Object.defineProperty(e, n, {get: v, set: s, enumerable: true, configurable: true});
}

function $parcel$interopDefault(a) {
  return a && a.__esModule ? a.default : a;
}

      var $parcel$global = globalThis;
    
var $parcel$modules = {};
var $parcel$inits = {};

var parcelRequire = $parcel$global["parcelRequire94c2"];

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

  $parcel$global["parcelRequire94c2"] = parcelRequire;
}

var parcelRegister = parcelRequire.register;
parcelRegister("d4t4A", function(module, exports) {
module.exports = JSON.parse("{\"name\":\"fixture\",\"items\":[1,2,3]}");
});

parcelRegister("l4t3B", function(module, exports) {
$parcel$export(module.exports, "value", () => $l4t3B$export$value, (v) => $l4t3B$export$value = v);
var $l4t3B$export$value;
console.log("eval late " + typeof module + " " + (this === module.exports));
$l4t3B$export$value = "late-" + require("path").sep.length + "-" + parcelRequire("d4t4A").items.length;
});

function $s4y$export$say(line) {
    console.log(line);
}
$s4y$export$say("eval say");
var $d4t4A = parcelRequire("d4t4A");
$s4y$export$say("json " + (0, $parcel$interopDefault($d4t4A)).name);
function $later() {
    return parcelRequire("l4t3B").value;
}
$s4y$export$say("later " + $later() + " " + $later());
$s4y$export$say("path " + $ext$path.posix.join("a", "b"));
Promise.resolve(parcelRequire("d4t4A")).then((lazy)=>$s4y$export$say("json lazy " + lazy.items.length));
