<script>
  let { seed = 2 } = $props();
  let box = $state(seed * 2);
  function make() { box = { n: 0 }; }
  function bump() { box.n++; }
</script>
<button class="a" onclick={make}>make</button>
<button class="b" onclick={bump}>bump</button>
<p>{typeof box === 'number' ? box : box.n}</p>
