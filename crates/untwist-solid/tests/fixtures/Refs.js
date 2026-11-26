import { template as _$template } from "solid-js/web";
import { use as _$use } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><input><div class=box></div><span>span`);
import { onMount } from "solid-js";
export default function Refs() {
  let input;
  let box;
  onMount(() => {
    input.value = "mounted";
    console.log(box.tagName);
  });
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling,
      _el$4 = _el$3.nextSibling;
    var _ref$ = input;
    typeof _ref$ === "function" ? _$use(_ref$, _el$2) : input = _el$2;
    _$use(el => box = el, _el$3);
    _$use(el => el.setAttribute("data-ref", "yes"), _el$4);
    return _el$;
  })();
}