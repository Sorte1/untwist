import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<table><thead><tr><th>name</th><th>age</th></tr></thead><tbody>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<tr><td></td><td>`);
import { For } from "solid-js";
export default function Table(props) {
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling;
    _$insert(_el$3, _$createComponent(For, {
      get each() {
        return props.rows;
      },
      children: row => (() => {
        var _el$4 = _tmpl$2(),
          _el$5 = _el$4.firstChild,
          _el$6 = _el$5.nextSibling;
        _$insert(_el$5, () => row.name);
        _$insert(_el$6, () => row.age);
        return _el$4;
      })()
    }));
    return _el$;
  })();
}