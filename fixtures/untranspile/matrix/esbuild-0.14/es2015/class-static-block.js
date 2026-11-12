var __defProp = Object.defineProperty;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __publicField = (obj, key, value) => {
  __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);
  return value;
};
const _Config = class {
};
let Config = _Config;
__publicField(Config, "defaults", { mode: "fast" });
__publicField(Config, "ready");
(() => {
  _Config.ready = Object.keys(_Config.defaults).length > 0;
  _Config.loaded = "yes";
})();
(() => {
  let local2 = 3;
  _Config.triple = local2 * 3;
})();
console.log(Config.ready, Config.loaded, Config.triple, typeof local);
