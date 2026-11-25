import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<textarea></textarea> <p> </p>`, 1);

export default function Textarea($$anchor) {
	let text = $.state('hello');
	var fragment = root();
	var textarea = $.first_child(fragment);

	$.remove_textarea_child(textarea);

	var p = $.sibling(textarea, 2);
	var text_1 = $.only_child(p, true);

	$.template_effect(() => $.set_text(text_1, $.get(text).length));
	$.bind_value(textarea, () => $.get(text), ($$value) => $.set(text, $$value));
	$.append($$anchor, fragment);
}