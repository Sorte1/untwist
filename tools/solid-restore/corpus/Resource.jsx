import { createResource, Suspense } from "solid-js";

const load = (id) => new Promise((resolve) => setTimeout(() => resolve({ id, name: "item " + id }), 5));

export default function Resource(props) {
  const [data] = createResource(() => props.id, load);
  return (
    <div>
      <Suspense fallback={<p>loading</p>}>
        <p>{data()?.name}</p>
      </Suspense>
      <span>{data.loading ? "busy" : "idle"}</span>
    </div>
  );
}
