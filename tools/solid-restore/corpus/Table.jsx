import { For } from "solid-js";

export default function Table(props) {
  return (
    <table>
      <thead>
        <tr>
          <th>name</th>
          <th>age</th>
        </tr>
      </thead>
      <tbody>
        <For each={props.rows}>
          {(row) => (
            <tr>
              <td>{row.name}</td>
              <td>{row.age}</td>
            </tr>
          )}
        </For>
      </tbody>
    </table>
  );
}
