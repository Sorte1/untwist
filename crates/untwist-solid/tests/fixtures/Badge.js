import { template as _$template } from "solid-js/web";
import { setAttribute as _$setAttribute } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<span class=badge>`);
export default function Badge(props) {
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, () => props.label, null);
    _$insert(_el$, () => props.children, null);
    _$effect(() => _$setAttribute(_el$, "data-tone", props.tone ?? "plain"));
    return _el$;
  })();
}