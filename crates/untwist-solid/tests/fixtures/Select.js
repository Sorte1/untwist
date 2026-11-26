import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><select></select><p>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<option>`);
import { createSignal, For } from "solid-js";
export default function Select() {
  const options = ["red", "green", "blue"];
  const [picked, setPicked] = createSignal("green");
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling;
    _el$2.addEventListener("change", e => setPicked(e.currentTarget.value));
    _$insert(_el$2, _$createComponent(For, {
      each: options,
      children: option => (() => {
        var _el$4 = _tmpl$2();
        _el$4.value = option;
        _$insert(_el$4, option);
        return _el$4;
      })()
    }));
    _$insert(_el$3, picked);
    _$effect(() => _el$2.value = picked());
    return _el$;
  })();
}