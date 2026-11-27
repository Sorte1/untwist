import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { setAttribute as _$setAttribute } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<i>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<div><p></p><button>inc`);
import { createSignal } from "solid-js";
function Kid(props) {
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, () => props.a);
    return _el$;
  })();
}
export default function F1Once() {
  const [n, setN] = createSignal(1);
  const f = k => n() * k;
  return (() => {
    var _el$2 = _tmpl$2(),
      _el$3 = _el$2.firstChild,
      _el$4 = _el$3.nextSibling;
    _$setAttribute(_el$3, "title", f(1));
    _$insert(_el$3, f(2));
    _$insert(_el$2, _$createComponent(Kid, {
      a: f(3)
    }), _el$4);
    _el$4.$$click = () => setN(n() + 1);
    return _el$2;
  })();
}
_$delegateEvents(["click"]);