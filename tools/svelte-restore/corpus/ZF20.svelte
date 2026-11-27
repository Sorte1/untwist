<script>
  let a = $state('A');
  let b = $state('');
  let on = $state(true);
  let items = $state([1, 2]);
  let h = $state('<u>u</u>');
  function t() { a += '!'; b = b ? '' : 'Q'; on = !on; items = [...items, 3]; }
</script>
<button class="go" onclick={t}>go</button>
<ul><li>
  &amp;{'\n'}<i>{a}</i> 
 <span> s </span></li></ul> <pre>&#32;{#if on} Y {:else} N {/if}	x{'  '}{b}&#9;&amp;&#32;</pre> <select><option>
x </option></select>
