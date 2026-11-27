import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<p>`);
const list = [];
class Reg {
  static {
    list.push("s");
  }
  static made = list.push("m");
}
export default function YModClass() {
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, () => list.join());
    return _el$;
  })();
}