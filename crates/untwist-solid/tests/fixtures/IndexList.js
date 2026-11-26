import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<ol>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<li>`);
import { createSignal, Index } from "solid-js";
export default function IndexList() {
  const [names, setNames] = createSignal(["x", "y"]);
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, _$createComponent(Index, {
      get each() {
        return names();
      },
      children: (name, i) => (() => {
        var _el$2 = _tmpl$2();
        _el$2.$$click = () => setNames(names().map((n, j) => j === i ? n + "!" : n));
        _$insert(_el$2, name);
        return _el$2;
      })()
    }));
    return _el$;
  })();
}
_$delegateEvents(["click"]);