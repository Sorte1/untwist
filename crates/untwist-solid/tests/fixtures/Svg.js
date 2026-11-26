import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { setAttribute as _$setAttribute } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><svg width=40 height=40 viewBox="0 0 40 40"><circle cx=20 cy=20 fill=red></circle><path d="M0 0L10 10"stroke-width=2></path><text x=2 y=38></text></svg><button>grow`);
import { createSignal } from "solid-js";
export default function Svg() {
  const [r, setR] = createSignal(5);
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.firstChild,
      _el$4 = _el$3.nextSibling,
      _el$5 = _el$4.nextSibling,
      _el$6 = _el$2.nextSibling;
    _$insert(_el$5, r);
    _el$6.$$click = () => setR(r() + 5);
    _$effect(() => _$setAttribute(_el$3, "r", r()));
    return _el$;
  })();
}
_$delegateEvents(["click"]);