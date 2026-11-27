export function createSignal(v) { console.log("mine"); return [() => "mine:" + v, () => {}]; }
export function onMount(f) { console.log("my-mount"); }
export function Show(props) { return "custom:" + (props.when ? "T" : "F"); }
export default function ZHelpers() { return null; }
