import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<p>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<div><button>toggle`);
import { createSignal, onMount, onCleanup, Show } from "solid-js";
function Ticker(props) {
  onMount(() => console.log("mount", props.name));
  onCleanup(() => console.log("cleanup", props.name));
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, () => props.name);
    return _el$;
  })();
}
export default function Lifecycle() {
  const [shown, setShown] = createSignal(true);
  return (() => {
    var _el$2 = _tmpl$2(),
      _el$3 = _el$2.firstChild;
    _$insert(_el$2, _$createComponent(Show, {
      get when() {
        return shown();
      },
      get children() {
        return _$createComponent(Ticker, {
          name: "tick"
        });
      }
    }), _el$3);
    _el$3.$$click = () => setShown(!shown());
    return _el$2;
  })();
}
_$delegateEvents(["click"]);