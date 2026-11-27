import { template as _$template, insert as _$insert, createComponent as _$createComponent, effect as _$effect, setAttribute as _$setAttribute, memo as _$memo } from "solid-js/web";
import { createSignal } from "solid-js";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p></p><p></p><p></p><i>`);
function Kid(props) { return [props.a, "|", props.b]; }
export default function PLook() {
  const [n, setN] = createSignal(1);
  const f = (k) => (console.log("f", k), n() * k);
  return (() => {
    var _el$ = _tmpl$(), _el$2 = _el$.firstChild, _el$3 = _el$2.nextSibling, _el$4 = _el$3.nextSibling, _el$5 = _el$4.nextSibling;
    _$insert(_el$2, f(2));
    _$insert(_el$3, () => f(3));
    _$insert(_el$4, _$createComponent(Kid, { a: f(4), get b() { return f(5); } }));
    _$setAttribute(_el$5, "title", f(6));
    _el$5.$$click = () => setN(n() + 1);
    _el$5.textContent = f(7);
    return _el$;
  })();
}
