import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<p>`);
const list = [];
function add(x) {
  list.push(x);
}
add(1);
add(2);
export default function ZModFn() {
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, () => list.join());
    return _el$;
  })();
}