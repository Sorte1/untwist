import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<b>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<p><button>inc`);
import { createSignal } from "solid-js";
const [g, setG] = createSignal(1);
function Kid() {
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, g);
    return _el$;
  })();
}
export default function ZModDestr() {
  return (() => {
    var _el$2 = _tmpl$2(),
      _el$3 = _el$2.firstChild;
    _$insert(_el$2, _$createComponent(Kid, {}), _el$3);
    _el$3.$$click = () => setG(v => v + 1);
    return _el$2;
  })();
}
_$delegateEvents(["click"]);