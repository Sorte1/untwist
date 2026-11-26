import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { spread as _$spread } from "solid-js/web";
import { mergeProps as _$mergeProps } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><button>`);
import { createSignal, splitProps } from "solid-js";
export default function SpreadAttrs(props) {
  const [local, rest] = splitProps(props, ["label"]);
  const [extra, setExtra] = createSignal({
    "data-x": "1"
  });
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild;
    _$spread(_el$2, _$mergeProps(rest, extra, {
      "class": "btn",
      "onClick": () => setExtra({
        "data-x": "2",
        title: "two"
      })
    }), false, true);
    _$insert(_el$2, () => local.label);
    return _el$;
  })();
}