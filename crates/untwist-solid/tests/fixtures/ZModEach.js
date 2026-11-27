import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<p>`);
const list = [];
const add = x => list.push(x);
[1, 2, 3].forEach(add);
export default function ZModEach() {
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, () => list.join());
    return _el$;
  })();
}