import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<p> </p>`);
var root_1 = $.from_html(`<!> <button>flip</button>`, 1);

export default function EachDestructure($$anchor) {
	let rows = $.proxy([{ id: 1, name: 'a' }, { id: 2, name: 'b' }]);
	var fragment = root_1();
	var node = $.first_child(fragment);

	$.each(node, 17, () => rows, ({ id, name }) => id, ($$anchor, $$item) => {
		let id = () => $.get($$item).id;
		let name = () => $.get($$item).name;
		var p = root();
		var text = $.only_child(p);

		$.template_effect(() => $.set_text(text, `${id() ?? ''}: ${name() ?? ''}`));
		$.append($$anchor, p);
	});

	var button = $.sibling(node, 2);

	$.delegated('click', button, () => rows.reverse());
	$.append($$anchor, fragment);
}

$.delegate(['click']);