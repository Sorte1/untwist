import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<div>a</div> <div>b</div> <p>c</p> <button>go</button>`, 1);

export default function ClassExpr($$anchor) {
	let active = $.state(false);
	let color = $.state('red');
	var fragment = root();
	var div = $.first_child(fragment);
	var div_1 = $.sibling(div, 2);
	var p = $.sibling(div_1, 2);
	var button = $.sibling(p, 2);

	$.template_effect(() => {
		$.set_class(div, 1, $.clsx($.get(active) ? 'on' : 'off'));
		$.set_class(div_1, 1, $.clsx(['base', $.get(active) && 'lit']));
		$.set_style(p, `color: ${$.get(color) ?? ''}; margin: 0`);
	});

	$.delegated('click', button, () => {
		$.set(active, !$.get(active));
		$.set(color, 'blue');
	});

	$.append($$anchor, fragment);
}

$.delegate(['click']);