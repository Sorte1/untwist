import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p class=base></p><span class=fixed>fixed</span><button>next`);
import { createSignal } from "solid-js";
export default function ClassList() {
  const [n, setN] = createSignal(0);
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling,
      _el$4 = _el$3.nextSibling;
    _$insert(_el$2, n);
    _el$4.$$click = () => setN(n() + 1);
    _$effect(_p$ => {
      var _v$ = !!(n() % 2 === 1),
        _v$2 = !!(n() % 2 === 0),
        _v$3 = !!(n() > 2);
      _v$ !== _p$.e && _el$2.classList.toggle("odd", _p$.e = _v$);
      _v$2 !== _p$.t && _el$2.classList.toggle("even", _p$.t = _v$2);
      _v$3 !== _p$.a && _el$2.classList.toggle("big", _p$.a = _v$3);
      return _p$;
    }, {
      e: undefined,
      t: undefined,
      a: undefined
    });
    return _el$;
  })();
}
_$delegateEvents(["click"]);