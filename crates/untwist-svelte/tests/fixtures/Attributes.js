import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<a class="link">link</a> <button aria-label="Action">act</button> <img src="/a.png" alt=""/>`, 1);

export default function Attributes($$anchor, $$props) {
	let href = $.prop($$props, 'href', 3, '/home'),
		disabled = $.prop($$props, 'disabled', 3, false);

	let title = $.state('tip');
	var fragment = root();
	var a = $.first_child(fragment);
	var button = $.sibling(a, 2);

	$.next(2);

	$.template_effect(() => {
		$.set_attribute(a, 'href', href());
		$.set_attribute(a, 'title', $.get(title));
		button.disabled = disabled();
	});

	$.delegated('click', button, () => $.set(title, 'changed'));
	$.append($$anchor, fragment);
}

$.delegate(['click']);