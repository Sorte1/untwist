import { template as _$template } from "solid-js/web";
import { setAttribute as _$setAttribute } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div class=some-long-class-name-to-force-wrapping data-a=another-long-attribute-value-here><p data-long=another-long-attribute-value-here-to-wrap-the-line-for-sure>x\ny</p><pre>`);
export default function F4Tpl(props) {
  const s = `x
y`;
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.firstChild,
      _el$4 = _el$2.nextSibling;
    _$insert(_el$2, () => `c
d ${props.x}`, _el$3);
    _$insert(_el$4, () => props.x + `
  two`);
    _$effect(() => _$setAttribute(_el$2, "title", `a
b ${props.x}`));
    return _el$;
  })();
}