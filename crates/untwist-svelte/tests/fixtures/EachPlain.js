import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<span> </span>`);
var root_1 = $.from_html(`<!> <button>add</button>`, 1);

export default function EachPlain($$anchor) {
	let words = $.proxy(['alpha', 'beta']);
	var fragment = root_1();
	var node = $.first_child(fragment);

	$.each(node, 17, () => words, $.index, ($$anchor, word) => {
		var span = root();
		var text = $.only_child(span, true);

		$.template_effect(() => $.set_text(text, $.get(word)));
		$.append($$anchor, span);
	});

	var button = $.sibling(node, 2);

	$.delegated('click', button, () => words.push('gamma'));
	$.append($$anchor, fragment);
}

$.delegate(['click']);