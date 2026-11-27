import { template as _$template } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { setAttribute as _$setAttribute } from "solid-js/web";
import { className as _$className } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { createSignal } from "solid-js";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p></p><button>inc`);
export default function Look2() {
  const [n, setN] = createSignal(1);
  return (() => {
    var _el$ = _tmpl$(), _el$2 = _el$.firstChild, _el$3 = _el$2.nextSibling;
    _el$3.$$click = () => setN(n() + 1);
    _$effect(_p$ => {
      var _v$ = "c" + n(), _v$2 = "t" + n();
      _v$ !== _p$.e && _$className(_el$2, _p$.e = _v$);
      _v$ !== _p$.t && _$setAttribute(_el$2, "title", _p$.t = _v$2);
      return _p$;
    }, { e: undefined, t: "c1" });
    return _el$;
  })();
}
_$delegateEvents(["click"]);
