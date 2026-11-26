<script>
  let a = $state('x');
</script>
<p> lead {a} trail </p>
<p>a  b
   c</p>
<span>one</span> <span>two</span>
<pre>
  first
    {a}
</pre>
<textarea>  text {a}
 more</textarea>
<p title="q&quot;uote {'{'} &amp; <" data-x='single"double'>&#123;&#125; &lt;b&gt; &nbsp;| &copy;</p>
<p>{'<b>'}{`back\`tick ${a}`}{"}"}</p>
<div title="{a} and {a}" class="k {a}">mixed</div>
<p>{@html '<i>raw</i>'}</p>
