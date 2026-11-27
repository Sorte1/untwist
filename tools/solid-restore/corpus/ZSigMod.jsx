import { createSignal, onMount, Show } from "./ZHelpers.jsx";
export default function ZSigMod(props) {
  const [n] = createSignal(4);
  onMount(() => console.log("mounted-arg"));
  return <div><p>{n()}</p><Show when={props.flag}>kid</Show></div>;
}
