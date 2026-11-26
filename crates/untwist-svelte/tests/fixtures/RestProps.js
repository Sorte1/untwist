import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var rest_excludes = new Set(['$$slots', '$$events', '$$legacy', 'label']);
var root = $.from_html(`<p> </p>`);

export default function RestProps($$anchor, $$props) {
	let rest = $.rest_props($$props, rest_excludes);
	var p = root();

	$.attribute_effect(p, () => ({ ...rest }));

	var text = $.only_child(p, true);

	$.template_effect(() => $.set_text(text, $$props.label));
	$.append($$anchor, p);
}