
      var $parcel$global = globalThis;
    
var $parcel$modules = {};
var $parcel$inits = {};

var parcelRequire = $parcel$global["parcelRequire9d85"];

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

  $parcel$global["parcelRequire9d85"] = parcelRequire;
}

var parcelRegister = parcelRequire.register;
parcelRegister("i9OKf", function(module, exports) {
module.exports = Promise.resolve(require("./lazy.0180685f.js")).then(()=>parcelRequire('9gTag'));

});

var $6c65364e62985f08$export$75f8c4fa4f93c4a6;
console.log("eval cjs");
$6c65364e62985f08$export$75f8c4fa4f93c4a6 = function(n) {
    return n * 2;
};


console.log("eval index " + (0, $6c65364e62985f08$export$75f8c4fa4f93c4a6)(21));

(parcelRequire("i9OKf")).then((lazy)=>console.log("lazy " + lazy.value));


