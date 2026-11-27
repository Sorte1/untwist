import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p></p><p>`);
function createSignal(v) {
  return [() => "fake" + v, () => {}];
}
function createEffect(f) {
  console.log("my effect");
}
function onMount(f) {
  console.log("my mount");
}
export default function RSig2() {
  const [a] = createSignal(1);
  createEffect(() => console.log("never"));
  onMount(() => console.log("never2"));
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling;
    _$insert(_el$2, a);
    _$insert(_el$3, a);
    return _el$;
  })();
}