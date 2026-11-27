import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p>|<!>|</p><button>inc`);
import { createSignal } from "solid-js";
import { produce, batch, unwrap } from "./YProdLib.jsx";
export default function YProd() {
  const [s, setS] = createSignal({
    n: 1
  });
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.firstChild,
      _el$5 = _el$3.nextSibling,
      _el$4 = _el$5.nextSibling,
      _el$6 = _el$2.nextSibling;
    _$insert(_el$2, () => s().n, _el$3);
    _$insert(_el$2, () => batch([1, 2, 3], 2).length, _el$5);
    _$insert(_el$2, () => unwrap({
      value: "v"
    }), null);
    _el$6.$$click = () => setS(produce(s(), d => {
      d.n++;
    }));
    return _el$;
  })();
}
_$delegateEvents(["click"]);