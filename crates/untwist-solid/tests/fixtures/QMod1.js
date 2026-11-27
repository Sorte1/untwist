import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { memo as _$memo } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<p>|<!>|<!>|`);
const table = {};
table.k = "v";
const list = [];
for (let i = 0; i < 3; i++) list.push(i * 2);
const cfg = {
  a: 1
};
Object.assign(cfg, {
  b: 2
});
Object.freeze(cfg);
export default function QMod1() {
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$5 = _el$2.nextSibling,
      _el$3 = _el$5.nextSibling,
      _el$6 = _el$3.nextSibling,
      _el$4 = _el$6.nextSibling;
    _$insert(_el$, () => table.k, _el$2);
    _$insert(_el$, () => list.join(), _el$5);
    _$insert(_el$, () => cfg.b, _el$6);
    _$insert(_el$, () => Object.isFrozen(cfg) ? "frozen" : "open", null);
    return _el$;
  })();
}