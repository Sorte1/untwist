import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { memo as _$memo } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<em>child `),
  _tmpl$2 = /*#__PURE__*/_$template(`<div class=parent><button>more`);
import { createSignal } from "solid-js";
import Badge from "./Badge.jsx";
export default function Parent() {
  const [n, setN] = createSignal(0);
  return (() => {
    var _el$ = _tmpl$2(),
      _el$4 = _el$.firstChild;
    _$insert(_el$, _$createComponent(Badge, {
      label: "static"
    }), _el$4);
    _$insert(_el$, _$createComponent(Badge, {
      get label() {
        return `count ${n()}`;
      },
      get tone() {
        return n() > 1 ? "hot" : undefined;
      },
      get children() {
        var _el$2 = _tmpl$(),
          _el$3 = _el$2.firstChild;
        _$insert(_el$2, n, null);
        return _el$2;
      }
    }), _el$4);
    _$insert(_el$, _$createComponent(Badge, {
      label: "text",
      children: "plain text"
    }), _el$4);
    _el$4.$$click = () => setN(n() + 1);
    return _el$;
  })();
}
_$delegateEvents(["click"]);