import { createSignal, For } from "solid-js";

export default function Select() {
  const options = ["red", "green", "blue"];
  const [picked, setPicked] = createSignal("green");
  return (
    <div>
      <select value={picked()} onChange={(e) => setPicked(e.currentTarget.value)}>
        <For each={options}>{(option) => <option value={option}>{option}</option>}</For>
      </select>
      <p>{picked()}</p>
    </div>
  );
}
