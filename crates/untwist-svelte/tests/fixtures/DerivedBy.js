import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<p> </p> <button>more</button>`, 1);

export default function DerivedBy($$anchor) {
	let numbers = $.proxy([1, 2, 3]);

	let total = $.derived(() => {
		let sum = 0;

		for (const n of numbers) sum += n;

		return sum;
	});

	var fragment = root();
	var p = $.first_child(fragment);
	var text = $.only_child(p);
	var button = $.sibling(p, 2);

	$.template_effect(($0) => $.set_text(text, `${$0 ?? ''} = ${$.get(total) ?? ''}`), [() => numbers.join(' + ')]);
	$.delegated('click', button, () => numbers.push(numbers.length + 1));
	$.append($$anchor, fragment);
}

$.delegate(['click']);