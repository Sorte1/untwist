import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { style as _$style } from "solid-js/web";
import { setStyleProperty as _$setStyleProperty } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p style=--gap:4px>styled</p><p style=margin:0;padding:2px>plain</p><p>string</p><button>grow`);
import { createSignal } from "solid-js";
export default function Style(props) {
  const [size, setSize] = createSignal(10);
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling,
      _el$4 = _el$3.nextSibling,
      _el$5 = _el$4.nextSibling;
    _el$5.$$click = () => setSize(size() + 5);
    _$effect(_p$ => {
      var _v$ = props.color,
        _v$2 = `${size()}px`,
        _v$3 = `width: ${size() * 2}px`;
      _v$ !== _p$.e && _$setStyleProperty(_el$2, "color", _p$.e = _v$);
      _v$2 !== _p$.t && _$setStyleProperty(_el$2, "font-size", _p$.t = _v$2);
      _p$.a = _$style(_el$4, _v$3, _p$.a);
      return _p$;
    }, {
      e: undefined,
      t: undefined,
      a: undefined
    });
    return _el$;
  })();
}
_$delegateEvents(["click"]);