import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<b>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<i>`),
  _tmpl$3 = /*#__PURE__*/_$template(`<div><button>inc`);
import { createSignal, createContext, useContext } from "solid-js";
console.log("module top 1");
const [g, setG] = createSignal((console.log("init g"), 1));
const Ctx = createContext((console.log("init ctx"), "d"));
const table = {};
table.k = (console.log("fill table"), "v");
console.log("module top 2");
function A() {
  return (() => {
    var _el$ = _tmpl$();
    _el$.$$click = () => setG(g() + 1);
    _$insert(_el$, g, null);
    _$insert(_el$, () => table.k, null);
    _$insert(_el$, () => useContext(Ctx), null);
    return _el$;
  })();
}
function B() {
  table.k += "!";
  return (() => {
    var _el$2 = _tmpl$2();
    _$insert(_el$2, () => g() * 2, null);
    _$insert(_el$2, () => table.k, null);
    return _el$2;
  })();
}
export default function PShared() {
  return (() => {
    var _el$3 = _tmpl$3(),
      _el$4 = _el$3.firstChild;
    _$insert(_el$3, _$createComponent(A, {}), _el$4);
    _$insert(_el$3, _$createComponent(B, {}), _el$4);
    _$insert(_el$3, _$createComponent(Ctx.Provider, {
      value: "p",
      get children() {
        return _$createComponent(A, {});
      }
    }), _el$4);
    _el$4.$$click = () => setG(g() + 1);
    return _el$3;
  })();
}
_$delegateEvents(["click"]);