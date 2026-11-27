import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<i>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<div><p>x</p><button class=inc>inc`);
import { createSignal } from "solid-js";
function Kid(props) {
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, () => props.a, null);
    _$insert(_el$, () => props.children, null);
    return _el$;
  })();
}
const tag = s => s[0] + "!";
export default function ZSeqClassOnce() {
  const [n, setN] = createSignal(1);
  let v = 3,
    w = 0;
  const key = "a";
  const rest = {
    lang: "en"
  };
  const f = k => {
    console.log("f", k);
    return n() * k;
  };
  const o = {
    a: 1,
    get g() {
      console.log("g");
      return n() * 10;
    }
  };
  return (() => {
    var _el$2 = _tmpl$2(),
      _el$3 = _el$2.firstChild,
      _el$4 = _el$3.nextSibling;
    _el$3.classList.toggle("on", !!(v, f(3)));
    _el$4.$$click = () => {
      v++;
      setN(n() + 1);
    };
    return _el$2;
  })();
}
_$delegateEvents(["click"]);