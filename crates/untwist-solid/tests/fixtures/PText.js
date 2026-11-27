import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p> </p><p></p><p></p><p>pre </p><p> post</p><p></p><p></p><p>lit1</p><p></p><button>inc`);
import { createSignal } from "solid-js";
export default function PText() {
  const [n, setN] = createSignal(1);
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.firstChild,
      _el$4 = _el$2.nextSibling,
      _el$5 = _el$4.nextSibling,
      _el$6 = _el$5.nextSibling,
      _el$7 = _el$6.firstChild,
      _el$8 = _el$6.nextSibling,
      _el$9 = _el$8.firstChild,
      _el$0 = _el$8.nextSibling,
      _el$1 = _el$0.nextSibling,
      _el$10 = _el$1.nextSibling,
      _el$11 = _el$10.firstChild,
      _el$13 = _el$10.nextSibling,
      _el$14 = _el$13.nextSibling;
    _el$4.textContent = "s" + n();
    _$insert(_el$5, n);
    _$insert(_el$6, n, null);
    _$insert(_el$8, n, _el$9);
    _$insert(_el$0, n, null);
    _$insert(_el$0, () => n() * 2, null);
    _$insert(_el$1, () => `tpl ${n()}`);
    _$insert(_el$10, true, null);
    _$insert(_el$10, null, null);
    _$insert(_el$10, undefined, null);
    _$insert(_el$10, [1, 2], null);
    _$insert(_el$13, () => n() + "", null);
    _$insert(_el$13, () => n().toString(), null);
    _$insert(_el$13, () => String(n()), null);
    _el$14.$$click = () => setN(n() + 1);
    _$effect(() => _el$3.data = n());
    return _el$;
  })();
}
_$delegateEvents(["click"]);