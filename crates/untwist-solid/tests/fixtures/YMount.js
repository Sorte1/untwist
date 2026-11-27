import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p>`);
import { createSignal } from "solid-js";
import { later } from "./YLaterLib.jsx";
export default function YMount() {
  const [n] = createSignal(1);
  later(() => console.log("later ran", n()));
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild;
    _$insert(_el$2, n);
    return _el$;
  })();
}