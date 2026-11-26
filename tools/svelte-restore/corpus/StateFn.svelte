<script>
  let box = $state(() => 1);
  function make() { box = { n: 0 }; }
  function bump() { box.n++; }
</script>
<button class="a" onclick={make}>make</button>
<button class="b" onclick={bump}>bump</button>
<p>{typeof box === 'function' ? 'fn' : box.n}</p>
