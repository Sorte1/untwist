<script>
  let active = $state(false);
  let color = $state('blue');
</script>

<div class:active class:big={!active} style:color>{active ? 'yes' : 'no'}</div>
<button onclick={() => { active = !active; color = 'green'; }}>flip</button>
