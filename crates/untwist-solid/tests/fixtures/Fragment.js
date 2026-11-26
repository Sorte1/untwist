import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { memo as _$memo } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<dt>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<dd>`),
  _tmpl$3 = /*#__PURE__*/_$template(`<dl>`),
  _tmpl$4 = /*#__PURE__*/_$template(`<button>double`);
import { createSignal } from "solid-js";
function Pair(props) {
  return [(() => {
    var _el$ = _tmpl$();
    _$insert(_el$, () => props.term);
    return _el$;
  })(), (() => {
    var _el$2 = _tmpl$2();
    _$insert(_el$2, () => props.children);
    return _el$2;
  })()];
}
export default function Fragment() {
  const [n, setN] = createSignal(1);
  return [(() => {
    var _el$3 = _tmpl$3();
    _$insert(_el$3, _$createComponent(Pair, {
      term: "one",
      children: "first"
    }), null);
    _$insert(_el$3, _$createComponent(Pair, {
      term: "two",
      get children() {
        return ["second ", _$memo(() => n())];
      }
    }), null);
    return _el$3;
  })(), "text after", (() => {
    var _el$4 = _tmpl$4();
    _el$4.$$click = () => setN(n() * 2);
    return _el$4;
  })()];
}
_$delegateEvents(["click"]);