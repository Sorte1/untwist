import { createSignal, Show } from "solid-js";

export default function ShowFallback() {
  const [user, setUser] = createSignal(null);
  return (
    <div>
      <Show when={user()} fallback={<p class="out">Logged out</p>}>
        <p class="in">Welcome {user().name}</p>
      </Show>
      <Show when={user()} keyed>
        {(u) => <span>{u.name.toUpperCase()}</span>}
      </Show>
      <button onClick={() => setUser(user() ? null : { name: "ada" })}>toggle</button>
    </div>
  );
}
