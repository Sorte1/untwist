globalThis.parcelRequire94c2 = (function () {
  var inits = {}, done = {};
  function req(id) {
    if (done[id]) return done[id].exports;
    var module = (done[id] = { exports: {} });
    inits[id].call(module.exports, module, module.exports);
    return module.exports;
  }
  req.register = function (id, init) { inits[id] = init; };
  return req;
})();

function $parcel$export(e, n, v, s) {
  Object.defineProperty(e, n, {get: v, set: s, enumerable: true, configurable: true});
}

      var $parcel$global = globalThis;
    var parcelRequire = $parcel$global["parcelRequire94c2"];
var parcelRegister = parcelRequire.register;
parcelRegister("c1", function(module, exports) {
$parcel$export(module.exports, "value", () => $c1$value);
console.log("eval c1");
const $c1$value = 7;
});

console.log("chunk " + parcelRequire("c1").value);
