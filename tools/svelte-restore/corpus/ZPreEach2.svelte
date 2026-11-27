<script>
  let items = $state([1, 2]);
  let on = $state(true);
</script>
<pre>a {#if on}yes{/if} b</pre>
<pre>{#each items as i}<b>{i}</b>{/each}</pre>
<div>k {#each items as i}{i},{/each}</div>
