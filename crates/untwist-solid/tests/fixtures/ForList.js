import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { setAttribute as _$setAttribute } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><ul></ul><button class=add>add</button><button class=clear>clear`),
  _tmpl$2 = /*#__PURE__*/_$template(`<li>empty`),
  _tmpl$3 = /*#__PURE__*/_$template(`<li>: `);
import { createSignal, For } from "solid-js";
export default function ForList() {
  const [items, setItems] = createSignal(["a", "b"]);
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling,
      _el$4 = _el$3.nextSibling;
    _$insert(_el$2, _$createComponent(For, {
      get each() {
        return items();
      },
      get fallback() {
        return _tmpl$2();
      },
      children: (item, index) => (() => {
        var _el$6 = _tmpl$3(),
          _el$7 = _el$6.firstChild;
        _$insert(_el$6, () => index() + 1, _el$7);
        _$insert(_el$6, item, null);
        _$effect(() => _$setAttribute(_el$6, "data-index", index()));
        return _el$6;
      })()
    }));
    _el$3.$$click = () => setItems([...items(), "c" + items().length]);
    _el$4.$$click = () => setItems([]);
    return _el$;
  })();
}
_$delegateEvents(["click"]);