import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<p> </p> <button>edit</button>`, 1);

export default function StateObject($$anchor) {
	let user = $.proxy({ name: 'Kim', tags: ['x'] });
	var fragment = root();
	var p = $.first_child(fragment);
	var text = $.only_child(p);
	var button = $.sibling(p, 2);

	$.template_effect(($0) => $.set_text(text, `${user.name ?? ''} has ${user.tags.length ?? ''} tags: ${$0 ?? ''}`), [() => user.tags.join(',')]);

	$.delegated('click', button, () => {
		user.name = 'Lee';
		user.tags.push('y');
	});

	$.append($$anchor, fragment);
}

$.delegate(['click']);