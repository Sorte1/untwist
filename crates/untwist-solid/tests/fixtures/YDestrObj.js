import { template as _$template } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<b>a`),
  _tmpl$2 = /*#__PURE__*/_$template(`<i>`),
  _tmpl$3 = /*#__PURE__*/_$template(`<div>`);
function makeCounter() {
  let n = 0;
  return {
    get: () => n,
    inc: () => ++n
  };
}
const {
  get,
  inc
} = makeCounter();
function Alpha() {
  inc();
  return _tmpl$();
}
function Beta() {
  return (() => {
    var _el$2 = _tmpl$2();
    _$insert(_el$2, get);
    return _el$2;
  })();
}
export default function YDestrObj() {
  return (() => {
    var _el$3 = _tmpl$3();
    _$insert(_el$3, _$createComponent(Alpha, {}), null);
    _$insert(_el$3, _$createComponent(Beta, {}), null);
    return _el$3;
  })();
}