import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { setAttribute as _$setAttribute } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div class=wrap>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<i>one`),
  _tmpl$3 = /*#__PURE__*/_$template(`<i>two `),
  _tmpl$4 = /*#__PURE__*/_$template(`<section><button>inc`);
import { children, createSignal } from "solid-js";
function Wrap(props) {
  const resolved = children(() => props.children);
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, resolved);
    _$effect(() => _$setAttribute(_el$, "data-count", resolved.toArray().length));
    return _el$;
  })();
}
export default function ChildrenHelper() {
  const [n, setN] = createSignal(1);
  return (() => {
    var _el$2 = _tmpl$4(),
      _el$6 = _el$2.firstChild;
    _$insert(_el$2, _$createComponent(Wrap, {
      get children() {
        return [_tmpl$2(), (() => {
          var _el$4 = _tmpl$3(),
            _el$5 = _el$4.firstChild;
          _$insert(_el$4, n, null);
          return _el$4;
        })()];
      }
    }), _el$6);
    _el$6.$$click = () => setN(n() + 1);
    return _el$2;
  })();
}
_$delegateEvents(["click"]);