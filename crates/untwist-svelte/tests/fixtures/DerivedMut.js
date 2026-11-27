import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<button class="a">n</button> <button class="b">o</button> <p> </p>`, 1);

export default function DerivedMut($$anchor) {
	let n = $.state(1);
	let d = $.derived(() => $.get(n) * 2);
	let list = $.derived(() => [$.get(n), $.get(n) + 1]);
	let fn = $.derived(() => () => $.get(n));

	function over() {
		$.set(d, 100);
	}

	var fragment = root();
	var button = $.first_child(fragment);
	var button_1 = $.sibling(button, 2);
	var p = $.sibling(button_1, 2);
	var text = $.only_child(p);

	$.template_effect(($0, $1) => $.set_text(text, `${$.get(d) ?? ''} ${$0 ?? ''} ${$1 ?? ''}`), [() => $.get(list).join(), () => $.get(fn)()]);
	$.delegated('click', button, () => $.update(n));
	$.delegated('click', button_1, over);
	$.append($$anchor, fragment);
}

$.delegate(['click']);