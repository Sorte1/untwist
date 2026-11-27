import { template as _$template } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p>`);
import { createSignal, onMount, Show } from "./ZHelpers.jsx";
export default function ZSigMod(props) {
  const [n] = createSignal(4);
  onMount(() => console.log("mounted-arg"));
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild;
    _$insert(_el$2, n);
    _$insert(_el$, _$createComponent(Show, {
      get when() {
        return props.flag;
      },
      children: "kid"
    }), null);
    return _el$;
  })();
}