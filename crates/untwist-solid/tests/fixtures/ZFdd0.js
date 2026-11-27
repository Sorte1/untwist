import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div title="a\nb V"><pre>x\n  y V\nz</pre><p>`);
export default function ZFdd0() {
  const v = "V";
  const mark = "﷐";
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling;
    _$insert(_el$3, () => mark.length, null);
    _$insert(_el$3, () => mark.charCodeAt(0), null);
    return _el$;
  })();
}