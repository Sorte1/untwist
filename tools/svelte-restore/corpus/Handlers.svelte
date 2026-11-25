<script>
  let seen = $state([]);

  function record(event, tag) {
    seen.push(tag + ':' + event.type);
  }
</script>

<button onclick={(event) => record(event, 'a')} ondblclick={(e) => record(e, 'b')}>x</button>
<div onclickcapture={(e) => record(e, 'cap')}><span>inner</span></div>
<ul>{#each seen as item}<li>{item}</li>{/each}</ul>
