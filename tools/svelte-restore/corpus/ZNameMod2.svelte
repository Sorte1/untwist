<script module>
  export function snapshot(value) { return Object.keys(value).length; }
  export function user_effect(f) { console.log('mine', f()); }
  export function proxy(v) { return { wrapped: v }; }
  export function derived(f) { return { d: f() }; }
</script>
<script>
  let form = $state({ a: 1, b: 2 });
  let saved = $state(0);
  const p = proxy(3);
  const d = derived(() => 4);
  user_effect(() => 1);
  function save() { saved = snapshot(form) + p.wrapped + d.d; }
</script>
<button onclick={save}>{saved}</button>
