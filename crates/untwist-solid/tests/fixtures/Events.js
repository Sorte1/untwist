import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { addEventListener as _$addEventListener } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><button class=a>a</button><button class=b>b</button><button class=c>c</button><button class=d>d</button><button class=e>e</button><p>`);
import { createSignal } from "solid-js";
export default function Events() {
  const [log, setLog] = createSignal("");
  const add = word => setLog(log() + word);
  const handler = (data, event) => add(data + event.type);
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling,
      _el$4 = _el$3.nextSibling,
      _el$5 = _el$4.nextSibling,
      _el$6 = _el$5.nextSibling,
      _el$7 = _el$6.nextSibling;
    _el$2.$$click = () => add("a");
    _el$3.$$click = handler;
    _el$3.$$clickData = "b";
    _$addEventListener(_el$4, "click", () => add("c"));
    _el$5.$$click = () => add("x");
    _el$5.$$mousedown = () => add("d");
    _el$6.addEventListener("focus", () => add("f"));
    _$insert(_el$7, log);
    return _el$;
  })();
}
_$delegateEvents(["click", "mousedown"]);