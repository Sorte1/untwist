import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<b>k`),
  _tmpl$2 = /*#__PURE__*/_$template(`<p>|`);
const k = "label";
function Kid() {
  return _tmpl$();
}
Kid[k] = "lbl";
export default function ZModComputed() {
  return (() => {
    var _el$2 = _tmpl$2(),
      _el$3 = _el$2.firstChild;
    _$insert(_el$2, _$createComponent(Kid, {}), _el$3);
    _$insert(_el$2, () => Kid.label, null);
    return _el$2;
  })();
}