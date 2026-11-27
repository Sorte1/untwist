import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { memo as _$memo } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { createSignal } from "solid-js";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p></p><button>inc`);
export default function Look3() {
  const [n, setN] = createSignal(0);
  return (() => {
    var _el$ = _tmpl$(), _el$2 = _el$.firstChild, _el$3 = _el$2.nextSibling;
    _el$3.$$click = () => setN(n() + 1);
    _$insert(_el$2, (() => {
      var _c$ = _$memo(() => n() > 1);
      return () => !_c$() ? "small" + n() : "big" + n();
    })());
    _$insert(_el$, (() => {
      var _c$ = _$memo(() => !!n());
      return () => _c$() === false ? "zero" : "nz";
    })(), _el$3);
    _$insert(_el$, (() => {
      var _c$ = _$memo(() => n() % 2);
      return () => _c$() && "odd" + (console.log("odd branch"), n());
    })(), _el$3);
    return _el$;
  })();
}
_$delegateEvents(["click"]);
