// Svelte refuses `bind:` on elements when it compiles for a custom renderer.
// Component libraries use it everywhere (`bind:this={ref}` on the root of
// nearly every component), so this preprocessor rewrites those bindings into
// attachments, which the custom renderer build does accept.
//
//   bind:this={x}     the node is assigned to `x` while the element exists
//   bind:value={x}    on <input>, <textarea>, <select>: `value={x}`, and `x`
//                     follows the element's `input` events
//   anything else     compiles, and warns when the element is created that
//                     the binding does nothing
//
// `bind:` on components is not touched: that is Svelte's own business and works.
// Runs in Node.

import { parse } from 'svelte/compiler';

/** Elements whose `value` the host reports with `input` events. */
const VALUE_ELEMENTS = new Set(['input', 'textarea', 'select']);

/**
 * Every element in a template, depth first.
 * @param {any} node
 * @param {(element: any) => void} visit
 */
function elements(node, visit) {
	if (node === null || typeof node !== 'object') return;
	if (Array.isArray(node)) {
		for (const child of node) elements(child, visit);
		return;
	}
	if (node.type === 'RegularElement' || node.type === 'SvelteElement') visit(node);
	for (const key of Object.keys(node)) {
		if (key !== 'metadata' && key !== 'parent') elements(node[key], visit);
	}
}

/**
 * What replaces one `bind:` directive. Kept on one line, so line numbers in
 * error messages stay right.
 * @param {string} tag
 * @param {string} name
 * @param {string} target the bound expression, as written
 * @param {string} file
 */
function replacement(tag, name, target, file) {
	if (name === 'this') {
		return `{@attach (__node) => { ${target} = __node; return () => { if (${target} === __node) ${target} = null; }; }}`;
	}
	if (name === 'value' && VALUE_ELEMENTS.has(tag)) {
		return (
			`value={${target}} ` +
			`{@attach (__node) => { const __input = () => { ${target} = __node.value; }; ` +
			`__node.addEventListener('input', __input); return () => __node.removeEventListener('input', __input); }}`
		);
	}
	const message = `sepal: bind:${name} on <${tag}> is not supported and does nothing (${file})`;
	return `{@attach () => { console.warn(${JSON.stringify(message)}); }}`;
}

/** @returns {import('svelte/compiler').PreprocessorGroup} */
export function element_bindings() {
	return {
		name: 'sepal:element-bindings',
		markup({ content, filename = 'component' }) {
			if (!content.includes('bind:')) return;
			let ast;
			try {
				ast = parse(content, { modern: true, filename });
			} catch {
				// Not ours to report: the compiler will, with a better message.
				return;
			}

			/** @type {{ start: number, end: number, text: string }[]} */
			const edits = [];
			elements(ast.fragment, (element) => {
				for (const attribute of element.attributes) {
					if (attribute.type !== 'BindDirective') continue;
					// `bind:x={get, set}` has no single thing to assign to; leave it for the compiler to refuse.
					if (attribute.expression.type === 'SequenceExpression') continue;
					const target = content.slice(attribute.expression.start, attribute.expression.end);
					const file = filename.split('/').slice(-2).join('/');
					edits.push({
						start: attribute.start,
						end: attribute.end,
						text: replacement(element.name, attribute.name, target, file)
					});
				}
			});
			if (edits.length === 0) return;

			let code = content;
			for (const edit of edits.sort((a, b) => b.start - a.start)) {
				code = code.slice(0, edit.start) + edit.text + code.slice(edit.end);
			}
			return { code };
		}
	};
}
