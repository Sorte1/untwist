import { template as _$template } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<b>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<i>`),
  _tmpl$3 = /*#__PURE__*/_$template(`<div>`);
const seen = [];
function remember(k) {
  seen.push(k);
  return seen.length;
}
function Alpha() {
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, () => remember("a"));
    return _el$;
  })();
}
function Beta() {
  return (() => {
    var _el$2 = _tmpl$2();
    _$insert(_el$2, () => remember("b"));
    return _el$2;
  })();
}
export default function F7Cache() {
  return (() => {
    var _el$3 = _tmpl$3();
    _$insert(_el$3, _$createComponent(Alpha, {}), null);
    _$insert(_el$3, _$createComponent(Beta, {}), null);
    _$insert(_el$3, () => remember("root"), null);
    return _el$3;
  })();
}