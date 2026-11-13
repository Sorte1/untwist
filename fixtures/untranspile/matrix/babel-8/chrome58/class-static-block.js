var _Config;
function _defineProperty(r, t, o) { return (t = _toPropertyKey(t)) in r ? Object.defineProperty(r, t, { value: o, enumerable: !0, configurable: !0, writable: !0 }) : r[t] = o, r; }
function _toPropertyKey(r) { var o = _toPrimitive(r, "string"); return "symbol" == typeof o ? o : o + ""; }
function _toPrimitive(t, e) { if ("object" != typeof t || !t) return t; var r; if ("undefined" != typeof Symbol && void 0 !== (r = t[Symbol.toPrimitive])) { var i = r.call(t, e || "default"); if ("object" != typeof i) return i; throw new TypeError("@@toPrimitive must return a primitive value."); } return ("string" === e ? String : Number)(t); }
class Config {}
_Config = Config;
_defineProperty(Config, "defaults", {
  mode: "fast"
});
_defineProperty(Config, "ready", void 0);
(() => {
  _Config.ready = Object.keys(_Config.defaults).length > 0;
  _Config.loaded = "yes";
})();
(() => {
  let local = 3;
  _Config.triple = local * 3;
})();
console.log(Config.ready, Config.loaded, Config.triple, typeof local);
