import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { memo as _$memo } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><button class=t>toggle</button><button class=n>inc`),
  _tmpl$2 = /*#__PURE__*/_$template(`<b>on`),
  _tmpl$3 = /*#__PURE__*/_$template(`<i>off`),
  _tmpl$4 = /*#__PURE__*/_$template(`<span>more than one`);
import { createSignal } from "solid-js";
export default function Ternary() {
  const [on, setOn] = createSignal(false);
  const [n, setN] = createSignal(0);
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling;
    _$insert(_el$, (() => {
      var _c$ = _$memo(() => !!on());
      return () => _c$() ? _tmpl$2() : _tmpl$3();
    })(), _el$2);
    _$insert(_el$, (() => {
      var _c$2 = _$memo(() => n() > 1);
      return () => _c$2() && _tmpl$4();
    })(), _el$2);
    _$insert(_el$, () => n() > 0 ? "positive" : "zero", _el$2);
    _el$2.$$click = () => setOn(!on());
    _el$3.$$click = () => setN(n() + 1);
    return _el$;
  })();
}
_$delegateEvents(["click"]);