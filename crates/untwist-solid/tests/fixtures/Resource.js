import { template as _$template } from "solid-js/web";
import { memo as _$memo } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<p>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<div><span>`),
  _tmpl$3 = /*#__PURE__*/_$template(`<p>loading`);
import { createResource, Suspense } from "solid-js";
const load = id => new Promise(resolve => setTimeout(() => resolve({
  id,
  name: "item " + id
}), 5));
export default function Resource(props) {
  const [data] = createResource(() => props.id, load);
  return (() => {
    var _el$ = _tmpl$2(),
      _el$3 = _el$.firstChild;
    _$insert(_el$, _$createComponent(Suspense, {
      get fallback() {
        return _tmpl$3();
      },
      get children() {
        var _el$2 = _tmpl$();
        _$insert(_el$2, () => data()?.name);
        return _el$2;
      }
    }), _el$3);
    _$insert(_el$3, () => data.loading ? "busy" : "idle");
    return _el$;
  })();
}