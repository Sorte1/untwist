import { template as _$template } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<b>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<p>`);
import { createSignal } from "solid-js";
const [g, setG] = createSignal(1);
const first = setG(5);
function Kid() {
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, g);
    return _el$;
  })();
}
export default function ZModDecl() {
  return (() => {
    var _el$2 = _tmpl$2();
    _$insert(_el$2, _$createComponent(Kid, {}));
    return _el$2;
  })();
}