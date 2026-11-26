import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { addEventListener as _$addEventListener } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<p>safe`),
  _tmpl$2 = /*#__PURE__*/_$template(`<div><button>arm`),
  _tmpl$3 = /*#__PURE__*/_$template(`<p class=err>`);
import { createSignal, ErrorBoundary, Show } from "solid-js";
function Bomb(props) {
  if (props.armed) throw new Error("boom");
  return _tmpl$();
}
export default function Boundary() {
  const [armed, setArmed] = createSignal(false);
  return (() => {
    var _el$2 = _tmpl$2(),
      _el$3 = _el$2.firstChild;
    _$insert(_el$2, _$createComponent(ErrorBoundary, {
      fallback: (err, reset) => (() => {
        var _el$4 = _tmpl$3();
        _$addEventListener(_el$4, "click", reset, true);
        _$insert(_el$4, () => err.message);
        return _el$4;
      })(),
      get children() {
        return _$createComponent(Show, {
          get when() {
            return armed();
          },
          get fallback() {
            return _$createComponent(Bomb, {
              armed: false
            });
          },
          get children() {
            return _$createComponent(Bomb, {
              armed: true
            });
          }
        });
      }
    }), _el$3);
    _el$3.$$click = () => setArmed(true);
    return _el$2;
  })();
}
_$delegateEvents(["click"]);