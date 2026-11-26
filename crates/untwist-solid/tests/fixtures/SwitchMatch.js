import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<p>zero`),
  _tmpl$2 = /*#__PURE__*/_$template(`<p>one`),
  _tmpl$3 = /*#__PURE__*/_$template(`<div><button>next`),
  _tmpl$4 = /*#__PURE__*/_$template(`<p>many`);
import { createSignal, Switch, Match } from "solid-js";
export default function SwitchMatch() {
  const [n, setN] = createSignal(0);
  return (() => {
    var _el$ = _tmpl$3(),
      _el$4 = _el$.firstChild;
    _$insert(_el$, _$createComponent(Switch, {
      get fallback() {
        return _tmpl$4();
      },
      get children() {
        return [_$createComponent(Match, {
          get when() {
            return n() === 0;
          },
          get children() {
            return _tmpl$();
          }
        }), _$createComponent(Match, {
          get when() {
            return n() === 1;
          },
          get children() {
            return _tmpl$2();
          }
        })];
      }
    }), _el$4);
    _el$4.$$click = () => setN(n() + 1);
    return _el$;
  })();
}
_$delegateEvents(["click"]);