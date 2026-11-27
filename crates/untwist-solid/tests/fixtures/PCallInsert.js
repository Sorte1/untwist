import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p></p><p></p><p></p><p></p><p>a<!>b<!>c</p><button>inc`);
import { createSignal } from "solid-js";
export default function PCallInsert() {
  const [n, setN] = createSignal(1);
  const f = k => n() * k;
  const g = () => n() + 100;
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling,
      _el$4 = _el$3.nextSibling,
      _el$5 = _el$4.nextSibling,
      _el$6 = _el$5.nextSibling,
      _el$7 = _el$6.firstChild,
      _el$0 = _el$7.nextSibling,
      _el$8 = _el$0.nextSibling,
      _el$1 = _el$8.nextSibling,
      _el$9 = _el$1.nextSibling,
      _el$10 = _el$6.nextSibling;
    _$insert(_el$2, f(2));
    _$insert(_el$3, () => f(3));
    _$insert(_el$4, g);
    _$insert(_el$5, g());
    _$insert(_el$6, g(), _el$0);
    _$insert(_el$6, g, _el$1);
    _el$10.$$click = () => setN(n() + 1);
    return _el$;
  })();
}
_$delegateEvents(["click"]);