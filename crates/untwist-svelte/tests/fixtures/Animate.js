import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';
import { flip } from 'svelte/animate';

var root = $.from_html(`<li> </li>`);
var root_1 = $.from_html(`<ul></ul> <button>flip</button>`, 1);

export default function Animate($$anchor) {
	let items = $.proxy([1, 2, 3]);
	var fragment = root_1();
	var ul = $.first_child(fragment);

	$.each(ul, 28, () => items, (item) => item, ($$anchor, item) => {
		var li = root();
		var text = $.only_child(li, true);

		$.template_effect(() => $.set_text(text, item));
		$.animation(li, () => flip, () => ({ duration: 0 }));
		$.append($$anchor, li);
	});

	$.reset(ul);

	var button = $.sibling(ul, 2);

	$.delegated('click', button, () => items.reverse());
	$.append($$anchor, fragment);
}

$.delegate(['click']);