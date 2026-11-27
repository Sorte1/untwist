import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { addEventListener as _$addEventListener } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<b>:`),
  _tmpl$2 = /*#__PURE__*/_$template(`<i>:<!>:`),
  _tmpl$3 = /*#__PURE__*/_$template(`<div><button>inc`);
import { createSignal } from "solid-js";
const makeStore = () => {
  const [v, set] = createSignal(0);
  return {
    v,
    bump: () => set(v() + 1)
  };
};
const store = makeStore();
const ids = new Map();
const idOf = k => {
  if (!ids.has(k)) ids.set(k, ids.size + 1);
  return ids.get(k);
};
function Alpha() {
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild;
    _$addEventListener(_el$, "click", store.bump, true);
    _$insert(_el$, () => store.v(), _el$2);
    _$insert(_el$, () => idOf("alpha"), null);
    return _el$;
  })();
}
function Beta() {
  return (() => {
    var _el$3 = _tmpl$2(),
      _el$4 = _el$3.firstChild,
      _el$6 = _el$4.nextSibling,
      _el$5 = _el$6.nextSibling;
    _$insert(_el$3, () => store.v(), _el$4);
    _$insert(_el$3, () => idOf("beta"), _el$6);
    _$insert(_el$3, () => idOf("alpha"), null);
    return _el$3;
  })();
}
export default function RCache2() {
  return (() => {
    var _el$7 = _tmpl$3(),
      _el$8 = _el$7.firstChild;
    _$insert(_el$7, _$createComponent(Alpha, {}), _el$8);
    _$insert(_el$7, _$createComponent(Beta, {}), _el$8);
    _el$8.$$click = () => store.bump();
    return _el$7;
  })();
}
_$delegateEvents(["click"]);