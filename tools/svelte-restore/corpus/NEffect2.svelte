<script module>
  function user_effect(f) { console.log('mine', f()); }
  function user_pre_effect(f) { console.log('mine-pre', f()); }
</script>
<script>
  let n = $state(1);
  user_effect(() => n);
  user_pre_effect(() => n + 1);
</script>
<button onclick={() => n++}>{n}</button>
