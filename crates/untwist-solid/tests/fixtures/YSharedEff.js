import { template as _$template } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<b>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<i>`),
  _tmpl$3 = /*#__PURE__*/_$template(`<div>`);
const a = [];
const b = {
  list: a
};
a.push(1, 2);
function Alpha() {
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, () => b.list.length);
    return _el$;
  })();
}
function Beta() {
  return (() => {
    var _el$2 = _tmpl$2();
    _$insert(_el$2, () => b.list.join());
    return _el$2;
  })();
}
export default function YSharedEff() {
  return (() => {
    var _el$3 = _tmpl$3();
    _$insert(_el$3, _$createComponent(Alpha, {}), null);
    _$insert(_el$3, _$createComponent(Beta, {}), null);
    return _el$3;
  })();
}