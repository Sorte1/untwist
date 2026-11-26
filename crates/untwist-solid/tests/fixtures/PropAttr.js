import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { setAttribute as _$setAttribute } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { getOwner as _$getOwner } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><input><my-element class=custom></my-element><button>more`, true, false, false);
import { createSignal } from "solid-js";
export default function PropAttr() {
  const [v, setV] = createSignal("x");
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling,
      _el$4 = _el$3.nextSibling;
    _el$3.items = [1, 2];
    _el$3._$owner = _$getOwner();
    _el$4.$$click = () => setV(v() + "y");
    _$effect(_p$ => {
      var _v$ = v(),
        _v$2 = v(),
        _v$3 = v();
      _v$ !== _p$.e && (_el$2.value = _p$.e = _v$);
      _v$2 !== _p$.t && _$setAttribute(_el$2, "data-v", _p$.t = _v$2);
      _v$3 !== _p$.a && _$setAttribute(_el$3, "size", _p$.a = _v$3);
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