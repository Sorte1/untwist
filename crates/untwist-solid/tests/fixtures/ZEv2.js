import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { addEventListener as _$addEventListener } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><button>b`);
const mk = k => {
  console.log(k);
  return () => {};
};
export default function ZEv2() {
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild;
    _el$2.$$keydown = mk("6");
    _el$2.$$keydownData = 1;
    _$addEventListener(_el$2, "mouseenter", mk("4"));
    _$addEventListener(_el$2, "foo", mk("3"));
    _$addEventListener(_el$2, "input", mk("2"), true);
    _$addEventListener(_el$2, "click", mk("1"), true);
    _el$2.addEventListener("bar", mk("5"), true);
    return _el$;
  })();
}
_$delegateEvents(["click", "input", "keydown"]);