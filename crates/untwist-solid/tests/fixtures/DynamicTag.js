import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<b class=red>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<div><button>swap`);
import { createSignal } from "solid-js";
import { Dynamic } from "solid-js/web";
function Red(props) {
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, () => props.text);
    return _el$;
  })();
}
export default function DynamicTag() {
  const [tag, setTag] = createSignal("h2");
  return (() => {
    var _el$2 = _tmpl$2(),
      _el$3 = _el$2.firstChild;
    _$insert(_el$2, _$createComponent(Dynamic, {
      get component() {
        return tag();
      },
      "class": "dyn",
      children: "heading"
    }), _el$3);
    _$insert(_el$2, _$createComponent(Dynamic, {
      component: Red,
      text: "red"
    }), _el$3);
    _el$3.$$click = () => setTag(tag() === "h2" ? "h3" : "h2");
    return _el$2;
  })();
}
_$delegateEvents(["click"]);