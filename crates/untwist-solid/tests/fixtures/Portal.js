import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<p class=portal>teleported`),
  _tmpl$2 = /*#__PURE__*/_$template(`<div><p>inside`);
import { Portal } from "solid-js/web";
export default function PortalTest() {
  return (() => {
    var _el$ = _tmpl$2(),
      _el$2 = _el$.firstChild;
    _$insert(_el$, _$createComponent(Portal, {
      get mount() {
        return document.querySelector("main");
      },
      get children() {
        return _tmpl$();
      }
    }), null);
    return _el$;
  })();
}