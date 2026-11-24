// A host written in plain JS. It applies op batches to a simple tree, which
// lets the renderer be tested without Rust, a GPU or a window.

import vm from 'node:vm';
import { OP, ROOT_ID } from '../src/protocol.js';

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
		this.nodes.set(ROOT_ID, { id: ROOT_ID, kind: 'element', name: 'body', attrs: {}, children: [], parent: null });
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
			__sb_host: {
				log: (level, message) => this.logs.push([level, message]),
				now: () => this.time
			}
		});
	}

	get root() {
		return this.nodes.get(ROOT_ID);
	}

	eval(code, filename = 'app.js') {
		vm.runInContext(code, this.context, { filename });
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
		return node.id === ROOT_ID;
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
