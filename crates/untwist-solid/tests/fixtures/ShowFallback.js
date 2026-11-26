import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<p class=in>Welcome `),
  _tmpl$2 = /*#__PURE__*/_$template(`<div><button>toggle`),
  _tmpl$3 = /*#__PURE__*/_$template(`<p class=out>Logged out`),
  _tmpl$4 = /*#__PURE__*/_$template(`<span>`);
import { createSignal, Show } from "solid-js";
export default function ShowFallback() {
  const [user, setUser] = createSignal(null);
  return (() => {
    var _el$ = _tmpl$2(),
      _el$4 = _el$.firstChild;
    _$insert(_el$, _$createComponent(Show, {
      get when() {
        return user();
      },
      get fallback() {
        return _tmpl$3();
      },
      get children() {
        var _el$2 = _tmpl$(),
          _el$3 = _el$2.firstChild;
        _$insert(_el$2, () => user().name, null);
        return _el$2;
      }
    }), _el$4);
    _$insert(_el$, _$createComponent(Show, {
      get when() {
        return user();
      },
      keyed: true,
      children: u => (() => {
        var _el$6 = _tmpl$4();
        _$insert(_el$6, () => u.name.toUpperCase());
        return _el$6;
      })()
    }), _el$4);
    _el$4.$$click = () => setUser(user() ? null : {
      name: "ada"
    });
    return _el$;
  })();
}
_$delegateEvents(["click"]);