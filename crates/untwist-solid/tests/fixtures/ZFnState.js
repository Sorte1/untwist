import { template as _$template } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<b>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<i>`),
  _tmpl$3 = /*#__PURE__*/_$template(`<div>`);
function next() {
  next.n = (next.n || 0) + 1;
  return next.n;
}
function Alpha() {
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, next);
    return _el$;
  })();
}
function Beta() {
  return (() => {
    var _el$2 = _tmpl$2();
    _$insert(_el$2, next);
    return _el$2;
  })();
}
export default function ZFnState() {
  return (() => {
    var _el$3 = _tmpl$3();
    _$insert(_el$3, _$createComponent(Alpha, {}), null);
    _$insert(_el$3, _$createComponent(Beta, {}), null);
    _$insert(_el$3, next, null);
    return _el$3;
  })();
}