<script>let a = $state('x'); let t = $state(true);</script>
<p><em>{' '}</em>|<b> {a} </b>|<i>{a}{' '}</i>|<u>{' '}{a}</u></p>
<p>a
{a}
b</p>
<div>  {#if t} yes {:else} no {/if}  </div>
<div>{#each [1, 2] as k} {k} {/each}</div>
<div>{#each [1, 2] as k}<span>{k}</span> {/each}</div>
<div><span>a</span>{' '}<span>b</span> <span>c</span>&#32;<span>d</span></div>
<button onclick={() => (t = !t)}>b</button>
