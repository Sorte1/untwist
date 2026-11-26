import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { memo as _$memo } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<form><input class=name><input class=agree type=checkbox><textarea></textarea><p> `);
import { createSignal } from "solid-js";
export default function Field() {
  const [text, setText] = createSignal("start");
  const [agree, setAgree] = createSignal(false);
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling,
      _el$4 = _el$3.nextSibling,
      _el$5 = _el$4.nextSibling,
      _el$6 = _el$5.firstChild;
    _el$.addEventListener("submit", e => e.preventDefault());
    _el$2.$$input = e => setText(e.currentTarget.value);
    _el$3.addEventListener("change", e => setAgree(e.currentTarget.checked));
    _$insert(_el$5, text, _el$6);
    _$insert(_el$5, () => agree() ? "yes" : "no", null);
    _$effect(() => _el$2.value = text());
    _$effect(() => _el$3.checked = agree());
    _$effect(() => _el$4.value = text());
    return _el$;
  })();
}
_$delegateEvents(["input"]);