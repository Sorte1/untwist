import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { addEventListener as _$addEventListener } from "solid-js/web";
import { use as _$use } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p>x`);
const mk = k => {
  console.log(k);
  return () => {};
};
export default function ZOrd5() {
  let el;
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild;
    _$addEventListener(_el$2, "input", mk("i"), true);
    _$addEventListener(_el$2, "click", mk("c"), true);
    var _ref$ = el;
    typeof _ref$ === "function" ? _$use(_ref$, _el$2) : el = _el$2;
    return _el$;
  })();
}
_$delegateEvents(["click", "input"]);