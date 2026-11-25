import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

const cell = ($$anchor, text = $.noop, n = $.noop) => {
	var td = root();
	var text_1 = $.only_child(td);

	$.template_effect(() => $.set_text(text_1, `${text() ?? ''} #${n() ?? ''}`));
	$.append($$anchor, td);
};

var root = $.from_html(`<td> </td>`);
var root_1 = $.from_html(`<table><tbody><tr><!><!></tr></tbody></table> <button>more</button>`, 1);

export default function Snippet($$anchor, $$props) {
	let label = $.prop($$props, 'label', 3, 'row');
	let count = $.state(1);
	var fragment = root_1();
	var table = $.first_child(fragment);
	var tbody = $.child(table);
	var tr = $.child(tbody);
	var node = $.child(tr);

	cell(node, label, () => $.get(count));

	var node_1 = $.sibling(node);

	cell(node_1, () => 'fixed', () => 2);
	$.reset(tr);
	$.reset(tbody);
	$.reset(table);

	var button = $.sibling(table, 2);

	$.delegated('click', button, () => $.update(count));
	$.append($$anchor, fragment);
}

$.delegate(['click']);