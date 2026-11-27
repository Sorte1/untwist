import { template as _$template } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<b>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<p>|<!>|`);
import { createSignal } from "solid-js";
const [g, setG] = createSignal(1);
setG(5);
const list = [];
for (let i = 0; i < 3; i++) list.push(i);
function Kid() {
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, g);
    return _el$;
  })();
}
Kid.label = "lbl";
export default function F2ModState() {
  return (() => {
    var _el$2 = _tmpl$2(),
      _el$3 = _el$2.firstChild,
      _el$5 = _el$3.nextSibling,
      _el$4 = _el$5.nextSibling;
    _$insert(_el$2, _$createComponent(Kid, {}), _el$3);
    _$insert(_el$2, () => Kid.label, _el$5);
    _$insert(_el$2, () => list.join(), null);
    return _el$2;
  })();
}