import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<article><header><h2></h2><small>by <!> on <time></time></small></header><div class=body><p>first <!> middle <!> last</p><p><span></span><span></span></p></div><footer><button>next`);
import { createSignal } from "solid-js";
export default function Nested(props) {
  const [n, setN] = createSignal(0);
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.firstChild,
      _el$4 = _el$3.nextSibling,
      _el$5 = _el$4.firstChild,
      _el$8 = _el$5.nextSibling,
      _el$6 = _el$8.nextSibling,
      _el$7 = _el$6.nextSibling,
      _el$9 = _el$2.nextSibling,
      _el$0 = _el$9.firstChild,
      _el$1 = _el$0.firstChild,
      _el$12 = _el$1.nextSibling,
      _el$10 = _el$12.nextSibling,
      _el$13 = _el$10.nextSibling,
      _el$11 = _el$13.nextSibling,
      _el$14 = _el$0.nextSibling,
      _el$15 = _el$14.firstChild,
      _el$16 = _el$15.nextSibling,
      _el$17 = _el$9.nextSibling,
      _el$18 = _el$17.firstChild;
    _$insert(_el$3, () => props.title);
    _$insert(_el$4, () => props.author, _el$8);
    _$insert(_el$7, () => props.date);
    _$insert(_el$0, n, _el$12);
    _$insert(_el$0, () => n() * 2, _el$13);
    _$insert(_el$15, n);
    _$insert(_el$16, () => n() + 1);
    _el$18.$$click = () => setN(n() + 1);
    return _el$;
  })();
}
_$delegateEvents(["click"]);