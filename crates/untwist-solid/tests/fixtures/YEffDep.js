import { template as _$template, effect as _$effect, setAttribute as _$setAttribute, className as _$className, delegateEvents as _$delegateEvents } from "solid-js/web";
import { createSignal } from "solid-js";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p>x</p><button>inc`);
export default function YEffDep() {
  const [n, setN] = createSignal(1);
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling;
    _el$3.$$click = () => setN(n() + 1);
    _$effect(_p$ => {
      var _v$ = "t" + n(),
        _v$2 = _v$ + "c";
      _v$ !== _p$.e && _$setAttribute(_el$2, "title", _p$.e = _v$);
      _v$2 !== _p$.t && _$className(_el$2, _p$.t = _v$2);
      return _p$;
    }, {
      e: undefined,
      t: undefined
    });
    return _el$;
  })();
}
_$delegateEvents(["click"]);