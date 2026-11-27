import { template as _$template } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { memo as _$memo } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<b class=mine>custom:<!>:`),
  _tmpl$2 = /*#__PURE__*/_$template(`<ol> items`),
  _tmpl$3 = /*#__PURE__*/_$template(`<div>`),
  _tmpl$4 = /*#__PURE__*/_$template(`<li>`);
function Show(props) {
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$4 = _el$2.nextSibling,
      _el$3 = _el$4.nextSibling;
    _$insert(_el$, () => props.when ? "T" : "F", _el$4);
    _$insert(_el$, () => props.children, null);
    return _el$;
  })();
}
function For(props) {
  return (() => {
    var _el$5 = _tmpl$2(),
      _el$6 = _el$5.firstChild;
    _$insert(_el$5, () => props.each.length, _el$6);
    return _el$5;
  })();
}
export default function RShow(props) {
  return (() => {
    var _el$7 = _tmpl$3();
    _$insert(_el$7, _$createComponent(Show, {
      get when() {
        return props.flag;
      },
      children: "kid"
    }), null);
    _$insert(_el$7, _$createComponent(For, {
      each: [1, 2, 3],
      children: x => (() => {
        var _el$8 = _tmpl$4();
        _$insert(_el$8, x);
        return _el$8;
      })()
    }), null);
    return _el$7;
  })();
}