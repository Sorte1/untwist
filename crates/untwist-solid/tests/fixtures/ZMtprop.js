import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { memo as _$memo } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<i>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<div><button>inc`);
import { createSignal } from "solid-js";
function Kid(props) {
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, () => props.children, null);
    _$insert(_el$, () => props.x, null);
    return _el$;
  })();
}
export default function ZMtprop() {
  const [n, setN] = createSignal(0);
  const a = () => {
    console.log("a");
    return n() + "a";
  };
  return (() => {
    var _el$2 = _tmpl$2(),
      _el$3 = _el$2.firstChild;
    _$insert(_el$2, _$createComponent(Kid, {
      get x() {
        return _$memo(() => !!n())() ? a() : "zero";
      }
    }), _el$3);
    _el$3.$$click = () => setN(n() + 1);
    return _el$2;
  })();
}
_$delegateEvents(["click"]);