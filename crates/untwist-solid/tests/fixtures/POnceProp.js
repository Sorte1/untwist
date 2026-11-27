import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<i>|<!>|`),
  _tmpl$2 = /*#__PURE__*/_$template(`<div><button>inc`);
import { createSignal } from "solid-js";
function Kid(props) {
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$4 = _el$2.nextSibling,
      _el$3 = _el$4.nextSibling;
    _$insert(_el$, () => props.a, _el$2);
    _$insert(_el$, () => props.b, _el$4);
    _$insert(_el$, () => props.c, null);
    return _el$;
  })();
}
export default function POnceProp() {
  const [n, setN] = createSignal(1);
  const f = k => n() * k;
  return (() => {
    var _el$5 = _tmpl$2(),
      _el$6 = _el$5.firstChild;
    _$insert(_el$5, _$createComponent(Kid, {
      a: f(2),
      get b() {
        return f(3);
      },
      c: n
    }), _el$6);
    _el$6.$$click = () => setN(n() + 1);
    return _el$5;
  })();
}
_$delegateEvents(["click"]);