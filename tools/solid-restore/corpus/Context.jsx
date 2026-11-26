import { createContext, useContext, createSignal } from "solid-js";

const Theme = createContext("light");

function Label() {
  const theme = useContext(Theme);
  return <span class={theme()}>{theme()}</span>;
}

export default function Context() {
  const [theme, setTheme] = createSignal("dark");
  return (
    <div>
      <Theme.Provider value={theme}>
        <Label />
      </Theme.Provider>
      <button onClick={() => setTheme(theme() === "dark" ? "light" : "dark")}>flip</button>
    </div>
  );
}
