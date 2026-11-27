import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { setAttribute as _$setAttribute } from "solid-js/web";
import { setStyleProperty as _$setStyleProperty } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<button>inc`);
import { createSignal } from "solid-js";
export default function F3AttrOrder() {
  const [n, setN] = createSignal(1);
  const log = k => (console.log(k), k + n());
  return (() => {
    var _el$ = _tmpl$();
    _el$.$$click = () => setN(n() + 1);
    _$effect(_p$ => {
      var _v$ = log("red"),
        _v$2 = log("t");
      _v$ !== _p$.e && _$setStyleProperty(_el$, "color", _p$.e = _v$);
      _v$2 !== _p$.t && _$setAttribute(_el$, "title", _p$.t = _v$2);
      return _p$;
    }, {
      e: undefined,
      t: undefined
    });
    return _el$;
  })();
}
_$delegateEvents(["click"]);