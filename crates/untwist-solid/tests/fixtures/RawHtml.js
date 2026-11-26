import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><div class=raw></div><div class=txt> </div><button>change`);
import { createSignal } from "solid-js";
export default function RawHtml(props) {
  const [html, setHtml] = createSignal("<b>bold</b>");
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling,
      _el$4 = _el$3.firstChild,
      _el$5 = _el$3.nextSibling;
    _el$5.$$click = () => setHtml("<i>it</i>");
    _$effect(_p$ => {
      var _v$ = html(),
        _v$2 = props.text;
      _v$ !== _p$.e && (_el$2.innerHTML = _p$.e = _v$);
      _v$2 !== _p$.t && (_el$4.data = _p$.t = _v$2);
      return _p$;
    }, {
      e: undefined,
      t: undefined
    });
    return _el$;
  })();
}
_$delegateEvents(["click"]);