import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<pre></pre> <pre></pre> <pre></pre> <pre>&#10;lit</pre> <textarea>&#10;lit</textarea> <textarea></textarea> <div><pre></pre></div>`, 1);

export default function ZPre2($$anchor) {
	let a = 'x';
	var fragment = root();
	var pre = $.first_child(fragment);

	pre.textContent = '\n\ntwo x';

	var pre_1 = $.sibling(pre, 2);

	pre_1.textContent = 'x\n';

	var pre_2 = $.sibling(pre_1, 2);

	pre_2.textContent = '\nx';

	var textarea = $.sibling(pre_2, 6);

	$.remove_textarea_child(textarea);
	$.set_value(textarea, '\nlit');

	var div = $.sibling(textarea, 2);
	var pre_3 = $.child(div);

	pre_3.textContent = '  in\n    deep x\n  ';
	$.reset(div);
	$.append($$anchor, fragment);
}