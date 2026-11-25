import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<span> <!> <button>pick</button></span>`);

export default function Badge($$anchor, $$props) {
	$.push($$props, true);

	let tone = $.prop($$props, 'tone', 3, 'info');
	var span = root();
	var text = $.child(span);
	var node = $.sibling(text);

	$.snippet(node, () => $$props.children ?? $.noop);

	var button = $.sibling(node, 2);

	$.reset(span);

	$.template_effect(() => {
		$.set_class(span, 1, `badge ${tone() ?? ''}`);
		$.set_text(text, `${$$props.label ?? ''} `);
	});

	$.delegated('click', button, () => $$props.onpick?.($$props.label));
	$.append($$anchor, span);
	$.pop();
}

$.delegate(['click']);