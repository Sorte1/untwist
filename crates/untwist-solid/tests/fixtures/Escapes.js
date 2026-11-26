import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div title="say &quot;hi&quot;"data-amp=a&amp;b><p>a &lt; b &amp;&amp; c &gt; d</p><p>\{braces} and &lt;tag></p><p>quote's "double" \`tick\` $\{dollar</p><p>&nbsp;spaced&nbsp;</p><p></p><p> `);
export default function Escapes(props) {
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling,
      _el$4 = _el$3.nextSibling,
      _el$5 = _el$4.nextSibling,
      _el$6 = _el$5.nextSibling;
    _$insert(_el$6, () => props.text);
    return _el$;
  })();
}