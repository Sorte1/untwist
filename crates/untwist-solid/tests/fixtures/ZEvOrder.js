import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { setAttribute as _$setAttribute } from "solid-js/web";
import { addEventListener as _$addEventListener } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><button>b`);
const mk = k => {
  console.log(k);
  return () => console.log("h" + k);
};
const s = k => {
  console.log(k);
  return k;
};
export default function ZEvOrder() {
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild;
    _$addEventListener(_el$2, "foo", mk("f"));
    _$addEventListener(_el$2, "click", mk("c"), true);
    _$setAttribute(_el$2, "title", s("t"));
    _$setAttribute(_el$2, "id", s("i"));
    return _el$;
  })();
}
_$delegateEvents(["click"]);