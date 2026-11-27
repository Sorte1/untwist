import { template as _$template } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><p></p><p>`);
export default function ZSigAfter() {
  const [n] = createSignal(4);
  onMount(() => console.log("mounted-arg"));
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling;
    _$insert(_el$2, n);
    _$insert(_el$3, () => For({
      each: [1, 2, 3]
    }));
    return _el$;
  })();
}
function createSignal(v) {
  console.log("mine");
  return [() => "mine:" + v, () => {}];
}
function onMount(f) {
  console.log("my-mount");
}
function For(props) {
  return props.each.length + " items";
}