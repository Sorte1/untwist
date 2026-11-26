import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(
	`<p></p> <p>a  b
   c</p> <span>one</span> <span>two</span> <pre></pre> <textarea></textarea> <p data-x="single&quot;double">&#123;&#125; &lt;b&gt; &nbsp;| &copy;</p> <p></p> <div>mixed</div> <p></p>`,
	1
);

export default function Space($$anchor) {
	let a = 'x';
	var fragment = root();
	var p = $.first_child(fragment);

	p.textContent = 'lead x trail';

	var pre = $.sibling(p, 8);

	pre.textContent = '\n  first\n    x\n';

	var textarea = $.sibling(pre, 2);

	$.remove_textarea_child(textarea);
	$.set_value(textarea, '  text x\n more');

	var p_1 = $.sibling(textarea, 2);

	$.set_attribute(p_1, 'title', 'q"uote { & <');

	var p_2 = $.sibling(p_1, 2);

	p_2.textContent = '<b>back`tick x}';

	var div = $.sibling(p_2, 2);

	$.set_attribute(div, 'title', 'x and x');
	$.set_class(div, 1, 'k x');

	var p_3 = $.sibling(div, 2);

	$.html(p_3, () => '<i>raw</i>', true);
	$.reset(p_3);
	$.append($$anchor, fragment);
}