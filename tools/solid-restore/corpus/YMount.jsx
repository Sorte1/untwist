import { createSignal } from "solid-js";
import { later } from "./YLaterLib.jsx";
export default function YMount() {
  const [n] = createSignal(1);
  later(() => console.log("later ran", n()));
  return <div><p>{n()}</p></div>;
}
