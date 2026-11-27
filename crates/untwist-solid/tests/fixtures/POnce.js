import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { className as _$className } from "solid-js/web";
import { setAttribute as _$setAttribute } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p></p><p></p><p>t</p><p>u</p><button>inc`);
import { createSignal } from "solid-js";
export default function POnce() {
  const [n, setN] = createSignal(1);
  const o = {
    get v() {
      return n();
    }
  };
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling,
      _el$4 = _el$3.nextSibling,
      _el$5 = _el$4.nextSibling,
      _el$6 = _el$5.nextSibling;
    _$insert(_el$2, o.v);
    _$insert(_el$3, () => o.v);
    _$setAttribute(_el$4, "title", o.v);
    _$className(_el$4, "c" + n());
    _el$6.$$click = () => setN(n() + 1);
    _$effect(() => _$setAttribute(_el$5, "title", o.v));
    return _el$;
  })();
}
_$delegateEvents(["click"]);