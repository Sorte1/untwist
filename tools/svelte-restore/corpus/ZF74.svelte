<script>
  let a = $state('A');
  let b = $state('');
  let on = $state(true);
  let items = $state([1, 2]);
  let h = $state('<u>u</u>');
  function t() { a += '!'; b = b ? '' : 'Q'; on = !on; items = [...items, 3]; }
</script>
<button class="go" onclick={t}>go</button>
<pre>{'  '}<em></em>{a}{b}{'  '}{@html h}{#each items as i}{i} {/each} 
 {b}</pre>

<select><option> {a} </option></select>
