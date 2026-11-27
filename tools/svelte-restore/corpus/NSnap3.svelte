<script module>
  export function snapshot(value) { return Object.keys(value).length; }
</script>
<script>
  let form = $state({ a: 1, b: 2 });
  let saved = $state(0);
  function save() { saved = snapshot(form); }
</script>
<button onclick={save}>{saved}</button>
