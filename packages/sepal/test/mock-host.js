// A host written in plain JS. It applies op batches to a simple tree, which
// lets the renderer be tested without Rust, a GPU or a window.

import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import vm from 'node:vm';
import { OP } from '../src/protocol.js';

// The page the Rust host would have parsed: <html><head></head><body></body></html>.
const HTML = 1;
const HEAD = 2;
const BODY = 3;

const ARITY = {
	[OP.CREATE_ELEMENT]: 2,
	[OP.CREATE_TEXT]: 2,
	[OP.CREATE_COMMENT]: 2,
	[OP.SET_TEXT]: 2,
	[OP.SET_ATTR]: 3,
	[OP.REMOVE_ATTR]: 2,
	[OP.INSERT]: 3,
	[OP.REMOVE]: 1,
	[OP.DROP]: 1,
	[OP.LISTEN]: 1,
	[OP.UNLISTEN]: 1,
	[OP.MEASURE]: 1,
	[OP.FOCUS]: 1,
	[OP.SCROLL]: 3,
	[OP.SCROLL_INTO_VIEW]: 3
};

export class MockHost {
	constructor() {
		this.nodes = new Map();
		const html = { id: HTML, kind: 'element', name: 'html', attrs: {}, children: [], parent: null };
		for (const [id, name] of [[HEAD, 'head'], [BODY, 'body']]) {
			const node = { id, kind: 'element', name, attrs: {}, children: [], parent: html };
			html.children.push(node);
			this.nodes.set(id, node);
		}
		this.nodes.set(HTML, html);
		this.listening = new Set();
		/** Ids of the nodes the app wants boxes for. */
		this.measured = new Set();
		/** The node with the keyboard focus, when the app moved it. */
		this.focused = null;
		/** `[node, left, top]` for every SCROLL, `[node, 'into view', block, inline]` for SCROLL_INTO_VIEW. */
		this.scrolls = [];
		this.time = 0;
		this.wake = -1;
		this.logs = [];
		this.stats = { batches: 0, ops: 0, last_ops: 0 };

		// A context with ECMAScript built-ins only: no DOM, no timers, no console.
		this.context = vm.createContext({
			// What the Rust host hands over before any script runs (its page.js).
			__sb_page: {
				ops: [OP.CREATE_ELEMENT, HTML, 0, OP.CREATE_ELEMENT, HEAD, 1, OP.INSERT, HTML, HEAD, 0, OP.CREATE_ELEMENT, BODY, 2, OP.INSERT, HTML, BODY, 0],
				strings: ['html', 'head', 'body'],
				next_id: BODY + 1
			},
			__sb_host: {
				log: (level, message) => this.logs.push([level, message]),
				now: () => this.time
			}
		});
	}

	/** `<body>`, where apps mount. */
	get root() {
		return this.nodes.get(BODY);
	}

	eval(code, filename = 'app.js') {
		vm.runInContext(code, this.context, { filename });
		this.settle();
	}

	/**
	 * Load an app folder as the Rust host does: the module scripts of its
	 * index.html, and what they import, by URL from the folder. Needs
	 * `node --experimental-vm-modules`.
	 * @param {string} dist
	 */
	async load(dist) {
		/** @type {Map<string, any>} */
		const modules = new Map();
		/** @param {string} url */
		const module_at = (url) => {
			let module = modules.get(url);
			if (module === undefined) {
				module = new /** @type {any} */ (vm).SourceTextModule(readFileSync(join(dist, new URL(url).pathname), 'utf8'), {
					context: this.context,
					identifier: url,
					initializeImportMeta: (/** @type {any} */ meta) => void (meta.url = url),
					importModuleDynamically: async (/** @type {string} */ specifier) => {
						const imported = module_at(new URL(specifier, url).href);
						await imported.link(link);
						await imported.evaluate();
						return imported;
					}
				});
				modules.set(url, module);
			}
			return module;
		};
		/** @param {string} specifier @param {any} referrer */
		const link = (specifier, referrer) => module_at(new URL(specifier, referrer.identifier).href);
		const page = readFileSync(join(dist, 'index.html'), 'utf8');
		for (const [, src] of page.matchAll(/<script type="module"[^>]*\ssrc="([^"]+)"/g)) {
			const module = module_at(new URL(src, 'http://app.localhost/').href);
			await module.link(link);
			await module.evaluate();
		}
		this.settle();
	}

	/** What the Rust host does after every entry into JS (microtasks have already run). */
	settle() {
		const batch = vm.runInContext('__sb.take()', this.context);
		if (batch) this.apply(batch);
		this.wake = vm.runInContext('__sb.next_timer()', this.context);
	}

	async flush_microtasks() {
		await new Promise((resolve) => setImmediate(resolve));
		this.settle();
	}

	apply({ ops, strings }) {
		this.stats.batches++;
		this.stats.last_ops = 0;
		for (let i = 0; i < ops.length; ) {
			const code = ops[i];
			const n = ARITY[code];
			if (n === undefined) throw new Error(`unknown opcode ${code} at ${i}`);
			const [a, b, c] = ops.subarray(i + 1, i + 1 + n);
			i += 1 + n;
			this.stats.ops++;
			this.stats.last_ops++;

			switch (code) {
				case OP.CREATE_ELEMENT:
					this.nodes.set(a, { id: a, kind: 'element', name: strings[b], attrs: {}, children: [], parent: null });
					break;
				case OP.CREATE_TEXT:
					this.nodes.set(a, { id: a, kind: 'text', data: strings[b], parent: null });
					break;
				case OP.CREATE_COMMENT:
					this.nodes.set(a, { id: a, kind: 'comment', data: strings[b], parent: null });
					break;
				case OP.SET_TEXT:
					this.node(a).data = strings[b];
					break;
				case OP.SET_ATTR:
					this.node(a).attrs[strings[b]] = strings[c];
					break;
				case OP.REMOVE_ATTR:
					delete this.node(a).attrs[strings[b]];
					break;
				case OP.INSERT: {
					const parent = this.node(a);
					const node = this.node(b);
					this.detach(node);
					if (c === 0) parent.children.push(node);
					else {
						const index = parent.children.indexOf(this.node(c));
						if (index === -1) throw new Error(`anchor ${c} is not a child of ${a}`);
						parent.children.splice(index, 0, node);
					}
					node.parent = parent;
					break;
				}
				case OP.REMOVE:
					this.detach(this.node(a));
					break;
				case OP.DROP: {
					const node = this.nodes.get(a);
					if (node) {
						if (this.attached(node)) throw new Error(`DROP of node ${a} that is still in the tree`);
						this.nodes.delete(a);
					}
					break;
				}
				case OP.LISTEN:
					this.listening.add(strings[a]);
					break;
				case OP.UNLISTEN:
					this.listening.delete(strings[a]);
					break;
				case OP.MEASURE:
					this.measured.add(this.node(a).id);
					break;
				case OP.FOCUS:
					this.focused = a === 0 ? null : this.node(a);
					break;
				case OP.SCROLL:
					this.scrolls.push([this.node(a), b, c]);
					break;
				case OP.SCROLL_INTO_VIEW:
					this.scrolls.push([this.node(a), 'into view', b, c]);
					break;
			}
		}
	}

	node(id) {
		const node = this.nodes.get(id);
		if (!node) throw new Error(`op refers to unknown node ${id}`);
		return node;
	}

	detach(node) {
		if (!node.parent) return;
		const siblings = node.parent.children;
		siblings.splice(siblings.indexOf(node), 1);
		node.parent = null;
	}

	attached(node) {
		while (node.parent) node = node.parent;
		return node.id === HTML;
	}

	dispatch(node, type, detail = null, bubbles = true) {
		if (!this.listening.has(type)) return 0;
		this.context.__args = [node.id, type, bubbles, detail];
		const flags = vm.runInContext('__sb.dispatch(...__args)', this.context);
		this.settle();
		return flags;
	}

	/**
	 * What the Rust host does at the end of a turn in which a box scrolled:
	 * a `scroll` event that does not bubble, sent whether anyone listens or
	 * not, with where the box is now. `metrics` is `[scrollLeft, scrollTop,
	 * scrollWidth, scrollHeight, clientWidth, clientHeight]`.
	 */
	scroll(node, metrics) {
		const names = ['scrollLeft', 'scrollTop', 'scrollWidth', 'scrollHeight', 'clientWidth', 'clientHeight'];
		this.context.__args = [node.id, 'scroll', false, Object.fromEntries(names.map((name, i) => [name, metrics[i]]))];
		vm.runInContext('__sb.dispatch(...__args)', this.context);
		this.settle();
	}

	/**
	 * What the Rust host does after a layout: tell the app the viewport size
	 * and the boxes that changed. `boxes` is a list of `[node, x, y, width, height]`.
	 */
	layout(width, height, boxes = []) {
		this.context.__args = [width, height, boxes.flatMap(([node, ...box]) => [node.id, ...box])];
		vm.runInContext('__sb.layout(...__args)', this.context);
		this.settle();
	}

	advance(ms) {
		this.time += ms;
		// The Rust host runs microtasks between timers; here they wait for
		// `flush_microtasks`, as everything does in this mock.
		while (vm.runInContext('__sb.timers()', this.context));
		this.settle();
	}

	// --- queries -------------------------------------------------------------

	find(predicate, node = this.root) {
		if (predicate(node)) return node;
		for (const child of node.children ?? []) {
			const found = this.find(predicate, child);
			if (found) return found;
		}
		return null;
	}

	by_id(id) {
		return this.find((node) => node.kind === 'element' && node.attrs.id === id);
	}

	text(node) {
		if (node.kind === 'text') return node.data;
		if (node.kind === 'comment') return '';
		return node.children.map((child) => this.text(child)).join('');
	}
}
