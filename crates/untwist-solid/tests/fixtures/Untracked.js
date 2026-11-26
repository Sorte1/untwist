import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p></p><p>static label</p><p></p><ul>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<li>`);
export default function Untracked(props) {
  const initial = props.start;
  const label = "static label";
  const list = [1, 2, 3];
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling,
      _el$4 = _el$3.nextSibling,
      _el$5 = _el$4.nextSibling;
    _$insert(_el$2, initial);
    _$insert(_el$4, () => list.map(x => x * 2).join(","));
    _$insert(_el$5, () => list.map(x => (() => {
      var _el$6 = _tmpl$2();
      _$insert(_el$6, x);
      return _el$6;
    })()));
    return _el$;
  })();
}