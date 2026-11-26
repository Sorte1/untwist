import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { mergeProps as _$mergeProps } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<section>`);
import { mergeProps } from "solid-js";
import Badge from "./Badge.jsx";
export default function SpreadChild(props) {
  const merged = mergeProps({
    tone: "soft"
  }, props);
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, _$createComponent(Badge, _$mergeProps(merged, {
      label: "first"
    })), null);
    _$insert(_el$, _$createComponent(Badge, _$mergeProps({
      label: "second"
    }, props)), null);
    return _el$;
  })();
}