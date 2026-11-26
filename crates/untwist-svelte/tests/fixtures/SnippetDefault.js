import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

const tag = ($$anchor, text = $.noop, $$arg1) => {
	let n = $.derived_safe_equal(() => $.fallback($$arg1?.(), 5));
	var b = root();
	var text_1 = $.only_child(b);

	$.template_effect(() => $.set_text(text_1, `${text() ?? ''}:${$.get(n) ?? ''}`));
	$.append($$anchor, b);
};

var root = $.from_html(`<b> </b>`);
var root_1 = $.from_html(`<!> <!> <button>more</button>`, 1);

export default function SnippetDefault($$anchor) {
	let count = $.state(1);
	var fragment = root_1();
	var node = $.first_child(fragment);

	tag(node, () => 'a');

	var node_1 = $.sibling(node, 2);

	tag(node_1, () => 'b', () => $.get(count));

	var button = $.sibling(node_1, 2);

	$.delegated('click', button, () => $.update(count));
	$.append($$anchor, fragment);
}

$.delegate(['click']);