import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p></p><button>inc`);
import { createSignal } from "solid-js";
export default function ZMemoAfter2() {
  const [n, setN] = createSignal(1);
  const w = wrap(() => n() * 2);
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling;
    _$insert(_el$2, w);
    _el$3.$$click = () => setN(n() + 1);
    return _el$;
  })();
}
const log = f => {
  console.log("wrapped");
  return f;
};
const wrap = fn => log(() => fn());
_$delegateEvents(["click"]);