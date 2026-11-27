import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { setAttribute as _$setAttribute } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { memo as _$memo } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p></p><p></p><button>inc`),
  _tmpl$2 = /*#__PURE__*/_$template(`<b>yes`),
  _tmpl$3 = /*#__PURE__*/_$template(`<i>no`),
  _tmpl$4 = /*#__PURE__*/_$template(`<u>two`),
  _tmpl$5 = /*#__PURE__*/_$template(`<s>big`),
  _tmpl$6 = /*#__PURE__*/_$template(`<em>zero`),
  _tmpl$7 = /*#__PURE__*/_$template(`<b>t`),
  _tmpl$8 = /*#__PURE__*/_$template(`<q>never`),
  _tmpl$9 = /*#__PURE__*/_$template(`<b>once`),
  _tmpl$0 = /*#__PURE__*/_$template(`<i>once-no`);
import { createSignal } from "solid-js";
export default function PCond() {
  const [n, setN] = createSignal(0);
  const log = (k, v) => (console.log(k, v), v);
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling,
      _el$4 = _el$3.nextSibling;
    _$insert(_el$, (() => {
      var _c$ = _$memo(() => !!log("a", n() > 0));
      return () => _c$() ? (() => {
        var _el$5 = _tmpl$2(),
          _el$6 = _el$5.firstChild;
        _$insert(_el$5, n, null);
        return _el$5;
      })() : _tmpl$3();
    })(), _el$2);
    _$insert(_el$, (() => {
      var _c$2 = _$memo(() => !!log("b", n() > 1));
      return () => _c$2() && _tmpl$4();
    })(), _el$2);
    _$insert(_el$, (() => {
      var _c$3 = _$memo(() => n() > 0);
      return () => _c$3() ? _$memo(() => n() > 1)() ? _tmpl$5() : "one" : null;
    })(), _el$2);
    _$insert(_el$, () => log("c", n()) || _tmpl$6(), _el$2);
    _$insert(_el$, () => n() > 1 ? "x" : "y", _el$2);
    _$insert(_el$2, (() => {
      var _c$4 = _$memo(() => !!n());
      return () => _c$4() ? _tmpl$7() : "f";
    })());
    _$insert(_el$3, () => n() && "lit");
    _$insert(_el$, () => n() ?? _tmpl$8(), _el$4);
    _$insert(_el$, n() > 0 ? _tmpl$9() : _tmpl$0(), _el$4);
    _el$4.$$click = () => setN(n() + 1);
    _$effect(() => _$setAttribute(_el$3, "title", n() ? "a" : "b"));
    return _el$;
  })();
}
_$delegateEvents(["click"]);