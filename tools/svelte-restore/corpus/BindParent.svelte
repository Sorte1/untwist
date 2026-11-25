<script>
  import Field from './Field.svelte';
  let n = $state(1);
</script>

<Field bind:value={n} />
<p>n is {n}</p>
