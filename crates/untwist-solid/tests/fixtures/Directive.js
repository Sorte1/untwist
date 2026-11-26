import { template as _$template } from "solid-js/web";
import { use as _$use } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p>marked</p><p>blue`);
import { createSignal } from "solid-js";
function highlight(el, value) {
  el.dataset.color = value();
}
export default function Directive() {
  const [color] = createSignal("yellow");
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling;
    _$use(highlight, _el$2, () => color());
    _$use(highlight, _el$3, () => "blue");
    return _el$;
  })();
}