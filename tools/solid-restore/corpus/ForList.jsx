import { createSignal, For } from "solid-js";

export default function ForList() {
  const [items, setItems] = createSignal(["a", "b"]);
  return (
    <div>
      <ul>
        <For each={items()} fallback={<li>empty</li>}>
          {(item, index) => (
            <li data-index={index()}>
              {index() + 1}: {item}
            </li>
          )}
        </For>
      </ul>
      <button class="add" onClick={() => setItems([...items(), "c" + items().length])}>add</button>
      <button class="clear" onClick={() => setItems([])}>clear</button>
    </div>
  );
}
