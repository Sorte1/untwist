<script>
  let box = $state.raw(null);
  function make() { box = { n: 0 }; }
  function bump() { box.n++; }
</script>
<button class="a" onclick={make}>make</button>
<button class="b" onclick={bump}>bump</button>
<p>{box ? box.n : 'none'}</p>
