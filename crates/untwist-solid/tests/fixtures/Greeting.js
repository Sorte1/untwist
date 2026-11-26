import { template as _$template } from "solid-js/web";
import { setAttribute as _$setAttribute } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<p class=greeting>Hello, <!>!`);
export default function Greeting(props) {
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$4 = _el$2.nextSibling,
      _el$3 = _el$4.nextSibling;
    _$insert(_el$, () => props.name, _el$4);
    _$effect(() => _$setAttribute(_el$, "title", props.title));
    return _el$;
  })();
}