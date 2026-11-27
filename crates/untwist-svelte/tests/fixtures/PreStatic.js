import 'svelte/internal/disclose-version';
import 'svelte/internal/flags/legacy';
import * as $ from 'svelte/internal/client';

var root = $.from_html(
	`<pre>
  first
</pre> <pre>

two</pre> <textarea>
x</textarea> <p><em></em><b></b><i></i> <u></u></p>`,
	1
);

export default function PreStatic($$anchor) {
	var fragment = root();
	var p = $.sibling($.first_child(fragment), 6);
	var em = $.child(p);

	em.textContent = ' ';

	var b = $.sibling(em);

	b.textContent = '  a  ';

	var i = $.sibling(b);
	var text = $.sibling(i);

	text.nodeValue = '   ';

	var u = $.sibling(text);

	u.textContent = '\n';
	$.reset(p);
	$.append($$anchor, fragment);
}