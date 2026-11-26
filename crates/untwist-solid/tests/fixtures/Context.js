import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { className as _$className } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<span>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<div><button>flip`);
import { createContext, useContext, createSignal } from "solid-js";
const Theme = createContext("light");
function Label() {
  const theme = useContext(Theme);
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, theme);
    _$effect(() => _$className(_el$, theme()));
    return _el$;
  })();
}
export default function Context() {
  const [theme, setTheme] = createSignal("dark");
  return (() => {
    var _el$2 = _tmpl$2(),
      _el$3 = _el$2.firstChild;
    _$insert(_el$2, _$createComponent(Theme.Provider, {
      value: theme,
      get children() {
        return _$createComponent(Label, {});
      }
    }), _el$3);
    _el$3.$$click = () => setTheme(theme() === "dark" ? "light" : "dark");
    return _el$2;
  })();
}
_$delegateEvents(["click"]);