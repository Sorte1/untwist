import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p>Count: </p><button>+1`);
import { createSignal } from "solid-js";
export default function Counter() {
  const [count, setCount] = createSignal(0);
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.firstChild,
      _el$4 = _el$2.nextSibling;
    _$insert(_el$2, count, null);
    _el$4.$$click = () => setCount(count() + 1);
    return _el$;
  })();
}
_$delegateEvents(["click"]);