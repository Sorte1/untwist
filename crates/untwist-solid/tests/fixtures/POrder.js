import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { setAttribute as _$setAttribute } from "solid-js/web";
import { spread as _$spread } from "solid-js/web";
import { mergeProps as _$mergeProps } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { use as _$use } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<i>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<div><span></span><button>inc`);
import { createSignal } from "solid-js";
function Kid(props) {
  console.log("kid", props.name);
  return (() => {
    var _el$ = _tmpl$();
    var _ref$ = props.ref;
    typeof _ref$ === "function" ? _$use(_ref$, _el$) : props.ref = _el$;
    _$insert(_el$, () => props.children);
    return _el$;
  })();
}
export default function POrder(props) {
  const [n, setN] = createSignal(1);
  const log = k => (console.log(k), k);
  let r1, r2;
  return (() => {
    var _el$2 = _tmpl$2(),
      _el$3 = _el$2.firstChild,
      _el$4 = _el$3.nextSibling;
    _$use(e => console.log("ref div", e.childNodes.length), _el$2);
    _$setAttribute(_el$2, "title", log("t1"));
    _$spread(_el$2, _$mergeProps(() => ({
      id: log("spread")
    }), {
      "lang": log("t2")
    }), false, true);
    _$insert(_el$2, () => log("c1"), _el$3);
    _$insert(_el$2, _$createComponent(Kid, {
      name: "k1",
      ref(r$) {
        var _ref$2 = r1;
        typeof _ref$2 === "function" ? _ref$2(r$) : r1 = r$;
      },
      get children() {
        return log("kc1");
      }
    }), _el$3);
    _el$3.$$click = () => console.log(r1.tagName, r2.tagName);
    var _ref$3 = r2;
    typeof _ref$3 === "function" ? _$use(_ref$3, _el$3) : r2 = _el$3;
    _$insert(_el$3, () => log("c2"));
    _$insert(_el$2, _$createComponent(Kid, {
      name: "k2",
      get children() {
        return n();
      }
    }), _el$4);
    _el$4.$$click = () => (r2.click(), setN(n() + 1));
    return _el$2;
  })();
}
_$delegateEvents(["click"]);