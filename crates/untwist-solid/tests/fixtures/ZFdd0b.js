import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<p>:`);
export default function ZFdd0b() {
  const mark = `a﷐b`;
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild;
    _$insert(_el$, () => mark.length, _el$2);
    _$insert(_el$, () => mark.charCodeAt(1), null);
    return _el$;
  })();
}