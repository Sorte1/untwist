import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<button> </button>`);

export default function Field($$anchor, $$props) {
	$.push($$props, true);

	let value = $.prop($$props, 'value', 15, 0);
	var button = root();
	var text = $.only_child(button, true);

	$.template_effect(() => $.set_text(text, value()));
	$.delegated('click', button, () => $.update_prop(value));
	$.append($$anchor, button);
	$.pop();
}

$.delegate(['click']);