import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { className as _$className } from "solid-js/web";
import { setAttribute as _$setAttribute } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><a target=_blank rel=noopener>link</a><input type=checkbox><span tabindex=0>state</span><div></div><button type=button>toggle`);
import { createSignal } from "solid-js";
export default function Attributes(props) {
  const [on, setOn] = createSignal(false);
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling,
      _el$4 = _el$3.nextSibling,
      _el$5 = _el$4.nextSibling,
      _el$6 = _el$5.nextSibling;
    _el$6.$$click = () => setOn(!on());
    _$effect(_p$ => {
      var _v$ = props.href,
        _v$2 = on() ? "on" : "off",
        _v$3 = props.locked,
        _v$4 = on() ? "active" : "idle",
        _v$5 = on() ? 1 : 0,
        _v$6 = `box ${props.kind}`,
        _v$7 = props.id,
        _v$8 = !on();
      _v$ !== _p$.e && _$setAttribute(_el$2, "href", _p$.e = _v$);
      _v$2 !== _p$.t && _$setAttribute(_el$2, "aria-label", _p$.t = _v$2);
      _v$3 !== _p$.a && (_el$3.disabled = _p$.a = _v$3);
      _v$4 !== _p$.o && _$className(_el$4, _p$.o = _v$4);
      _v$5 !== _p$.i && _$setAttribute(_el$4, "data-state", _p$.i = _v$5);
      _v$6 !== _p$.n && _$className(_el$5, _p$.n = _v$6);
      _v$7 !== _p$.s && _$setAttribute(_el$5, "id", _p$.s = _v$7);
      _v$8 !== _p$.h && (_el$5.hidden = _p$.h = _v$8);
      return _p$;
    }, {
      e: undefined,
      t: undefined,
      a: undefined,
      o: undefined,
      i: undefined,
      n: undefined,
      s: undefined,
      h: undefined
    });
    _$effect(() => _el$3.checked = on());
    return _el$;
  })();
}
_$delegateEvents(["click"]);