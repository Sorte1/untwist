import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<p>`);
const routes = [];
function addRoute(name) {
  const r = {
    name
  };
  routes.push(r);
  return r;
}
export const home = addRoute("home");
export const about = addRoute("about");
export default function ZModDecl2() {
  return (() => {
    var _el$ = _tmpl$();
    _$insert(_el$, () => routes.map(r => r.name).join());
    return _el$;
  })();
}