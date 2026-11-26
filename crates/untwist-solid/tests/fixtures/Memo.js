import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<p> + <!> = <button>both`);
import { createSignal, createMemo, createEffect, on, untrack, batch } from "solid-js";
export default function Memo() {
  const [a, setA] = createSignal(1);
  const [b, setB] = createSignal(2);
  const sum = createMemo(() => a() + b());
  createEffect(() => console.log("sum", sum()));
  createEffect(on(a, (value, prev) => console.log("a", value, prev), {
    defer: true
  }));
  createEffect(() => console.log("b only", b(), untrack(a)));
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$5 = _el$2.nextSibling,
      _el$3 = _el$5.nextSibling,
      _el$4 = _el$3.nextSibling;
    _$insert(_el$, a, _el$2);
    _$insert(_el$, b, _el$5);
    _$insert(_el$, sum, _el$4);
    _el$4.$$click = () => batch(() => {
      setA(a() + 1);
      setB(b() + 1);
    });
    return _el$;
  })();
}
_$delegateEvents(["click"]);