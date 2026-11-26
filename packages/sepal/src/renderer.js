// The Svelte custom renderer.
//
// Svelte reads the tree synchronously (getParent, getFirstChild, getNextSibling),
// so every node is mirrored here as a small JS object with sibling pointers.
// Writes are NOT sent to the host one by one: they are appended to an op buffer
// that the host pulls once per turn with `take_batch()`. No call crosses the
// JS/Rust boundary while Svelte is rendering.

// Before Svelte: its runtime keeps engine functions it finds when it loads
// (`Object.getOwnPropertyDescriptor`), and shims.js fixes one in QuickJS.
// Whichever module loads Svelte first, this one comes before it.
import './shims.js';
import { createRenderer } from 'svelte/renderer';
import { OP, DISPATCH_PREVENT_DEFAULT, DISPATCH_STOP_PROPAGATION } from './protocol.js';

const ELEMENT = 'element';
const TEXT = 'text';
const COMMENT = 'comment';
const FRAGMENT = 'fragment';

// ---------------------------------------------------------------------------
// Op buffer
// ---------------------------------------------------------------------------

/** @type {number[]} */
let ops = [];
/** @type {string[]} */
let strings = [];
/** @type {Map<string, number>} */
let string_ids = new Map();

/**
 * Intern a string for the current batch and return its index.
 * @param {string} value
 */
function str(value) {
	let index = string_ids.get(value);
	if (index === undefined) {
		index = strings.length;
		strings.push(value);
		string_ids.set(value, index);
	}
	return index;
}

/**
 * Hand the pending ops to the host and start a new batch.
 * Returns `null` when nothing changed, so an idle app does no work at all.
 * @returns {{ ops: Int32Array, strings: string[] } | null}
 */
export function take_batch() {
	if (ops.length === 0) return null;
	const batch = { ops: new Int32Array(ops), strings };
	ops = [];
	strings = [];
	string_ids = new Map();
	return batch;
}

// ---------------------------------------------------------------------------
// Nodes
// ---------------------------------------------------------------------------

/**
 * The page the host parsed from index.html, handed over once, before any
 * script ran (`page.js` in the host): its nodes as ops in this file's own
 * vocabulary, numbered from 1, and the first id this side may allocate.
 * `held` are the nodes a script took before this module loaded.
 * @type {{ ops: number[], strings: string[], next_id: number, held?: Map<number, object> }}
 */
const page = /** @type {any} */ (globalThis).__sb_page;
if (page === undefined) throw new Error('sepal: there is no page to render into. Apps run in calyx, the Sepal host, which loads them from index.html.');

let next_id = page.next_id;

export class BlitzNode {
	/**
	 * @param {'element' | 'text' | 'comment' | 'fragment'} type
	 * @param {number} id 0 for fragments, which never exist on the host
	 */
	constructor(type, id) {
		this.type = type;
		/** The id the host knows this node by. Not `id`: that is the HTML attribute, which the DOM layer exposes. */
		this.node_id = id;
		/** @type {BlitzNode | null} */
		this.parent = null;
		/** @type {BlitzNode | null} */
		this.first = null;
		/** @type {BlitzNode | null} */
		this.last = null;
		/** @type {BlitzNode | null} */
		this.prev = null;
		/** @type {BlitzNode | null} */
		this.next = null;
		/** element name */
		this.name = '';
		/** text or comment data */
		this.data = '';
		/** @type {Map<string, string> | null} */
		this.attrs = null;
		/** @type {Map<string, Function[]> | null} */
		this.listeners = null;
		/** current value of form controls, kept in sync by `input` events */
		/** @type {any} */
		this.value = undefined;
		/** @type {boolean | undefined} */
		this.checked = undefined;
	}

	// A few DOM-flavoured conveniences so handlers written for the web keep working.
	get nodeName() {
		return this.name.toUpperCase();
	}
	get parentNode() {
		return this.parent;
	}
	/** @param {string} name */
	getAttribute(name) {
		return this.attrs?.get(name) ?? null;
	}
	/**
	 * Listen on this node, as on a DOM element. Code that holds a node
	 * (an attachment, an action, a `bind:this`) has no other way to.
	 * @param {string} type
	 * @param {Function} handler
	 */
	addEventListener(type, handler) {
		add_listener(this, type, handler);
	}
	/** @param {string} type @param {Function} handler */
	removeEventListener(type, handler) {
		remove_listener(this, type, handler);
	}

	// Scrolling. The host says where a box is scrolled to with a `scroll`
	// event, at the end of the turn that moved it; a write goes to the host
	// as an op, and reads back at once, as in a browser.
	get scrollTop() {
		return scroll_positions.get(this)?.scrollTop ?? 0;
	}
	set scrollTop(value) {
		scroll_to(this, this.scrollLeft, value);
	}
	get scrollLeft() {
		return scroll_positions.get(this)?.scrollLeft ?? 0;
	}
	set scrollLeft(value) {
		scroll_to(this, value, this.scrollTop);
	}
	/** @param {number | { left?: number, top?: number }} [x] @param {number} [y] */
	scrollTo(x, y) {
		const [left, top] = scroll_arguments(x, y);
		scroll_to(this, left ?? this.scrollLeft, top ?? this.scrollTop);
	}
	/** @param {number | { left?: number, top?: number }} [x] @param {number} [y] */
	scrollBy(x, y) {
		const [left, top] = scroll_arguments(x, y);
		scroll_to(this, this.scrollLeft + (Number(left) || 0), this.scrollTop + (Number(top) || 0));
	}
	/**
	 * Scroll the boxes around this node until it is in view. Where they end up
	 * is known once the host has done it, at the end of the turn.
	 * @param {boolean | { block?: string, inline?: string }} [options]
	 */
	scrollIntoView(options) {
		const { block = 'start', inline = 'nearest' } =
			typeof options === 'object' && options !== null ? options : { block: options === false ? 'end' : 'start' };
		if (this.node_id !== 0) ops.push(OP.SCROLL_INTO_VIEW, this.node_id, ALIGN[block] ?? 0, ALIGN[inline] ?? 3);
	}
}

/** `block` and `inline` of `scrollIntoView()`, as the host numbers them. @type {Record<string, number>} */
const ALIGN = { start: 0, center: 1, end: 2, nearest: 3 };

/**
 * Where each box that has scrolled is, as the host last reported it (the
 * detail of its `scroll` event) or the app last wrote it.
 * `{ scrollLeft, scrollTop, scrollWidth, scrollHeight, clientWidth, clientHeight }`;
 * only the first two until the host has reported.
 * @type {WeakMap<BlitzNode, Record<string, number>>}
 */
const scroll_positions = new WeakMap();

/**
 * What the host last said about how a box scrolls, if it has.
 * @param {BlitzNode} node
 */
export const scroll_state = (node) => scroll_positions.get(node);

/** `scrollTo(x, y)` or `scrollTo({ left, top })`. @param {any} x @param {any} y @returns {any[]} */
const scroll_arguments = (x, y) => (typeof x === 'object' && x !== null ? [x.left, x.top] : [x, y]);

/**
 * @param {BlitzNode} node
 * @param {any} left
 * @param {any} top
 */
function scroll_to(node, left, top) {
	if (node.node_id === 0) return;
	let state = scroll_positions.get(node);
	if (state === undefined) scroll_positions.set(node, (state = { scrollLeft: 0, scrollTop: 0 }));
	// Ops are 32-bit integers.
	/** @param {any} value */
	const integer = (value) => Math.min(2 ** 31 - 1, Math.max(0, Math.round(Number(value) || 0)));
	left = integer(left);
	top = integer(top);
	// What reads back is clamped to the range the host last reported. The host
	// gets the value asked for: what the same turn adds to the box may make
	// room for it. It clamps to the range there is then, and reports that.
	/** @param {number} value @param {number | undefined} size @param {number | undefined} client */
	const clamp = (value, size, client) => (size === undefined || client === undefined ? value : Math.min(value, Math.max(0, size - client)));
	state.scrollLeft = clamp(left, state.scrollWidth, state.clientWidth);
	state.scrollTop = clamp(top, state.scrollHeight, state.clientHeight);
	ops.push(OP.SCROLL, node.node_id, left, top);
}

const has_weak_refs =
	typeof WeakRef === 'function' && typeof FinalizationRegistry === 'function';

/**
 * id -> element, used to find the target of an event coming from the host.
 * Entries are weak so that detached subtrees can be collected; when one is,
 * the finalizer tells the host to free its copy too.
 * @type {Map<number, WeakRef<BlitzNode> | BlitzNode>}
 */
const elements = new Map();

const finalizer = has_weak_refs
	? new FinalizationRegistry((/** @type {number} */ id) => {
			if (by_tag !== null) by_tag.get(/** @type {any} */ (elements.get(id))?.tag)?.delete(id);
			elements.delete(id);
			ops.push(OP.DROP, id);
		})
	: null;

/**
 * The ids of the elements alive, attached or not, by lower-case tag name:
 * the DOM layer's selectors try only these when a selector ends in a tag
 * (`meta[property=csp-nonce]`, `link[href=…]`, `title`), instead of walking
 * the page. Made the first time it is asked for, so an app without the DOM
 * layer never pays for it; after that, each element is added when it is made
 * and deleted when it is collected. The tag rides on the element's `WeakRef`,
 * for the finalizer, which gets only the id.
 * @type {Map<string, Set<number>> | null}
 */
let by_tag = null;

/** @param {BlitzNode} node @param {any} entry its entry in `elements` */
function index_tag(node, entry) {
	const tag = node.name.toLowerCase();
	if (has_weak_refs) entry.tag = tag;
	const ids = /** @type {Map<string, Set<number>>} */ (by_tag).get(tag);
	if (ids === undefined) /** @type {Map<string, Set<number>>} */ (by_tag).set(tag, new Set([node.node_id]));
	else ids.add(node.node_id);
}

/**
 * The elements named `name` (lower case) that are alive, attached or not, in
 * no particular order; `null` if there are more than `limit`.
 * @param {string} name
 * @param {number} limit
 * @returns {BlitzNode[] | null}
 */
export function elements_named(name, limit) {
	if (by_tag === null) {
		by_tag = new Map();
		for (const [id, entry] of elements) {
			const node = lookup(id);
			if (node !== null) index_tag(node, entry);
		}
	}
	const ids = by_tag.get(name);
	if (ids === undefined) return [];
	if (ids.size > limit) return null;
	/** @type {BlitzNode[]} */
	const nodes = [];
	for (const id of ids) {
		const node = lookup(id);
		if (node !== null) nodes.push(node);
	}
	return nodes;
}

/** @param {'element' | 'text' | 'comment'} type */
function create_node(type) {
	const node = new BlitzNode(type, next_id++);
	finalizer?.register(node, node.node_id);
	return node;
}

/**
 * Ask the host to report where this node is after every layout that moves or
 * resizes it. The answer arrives through `host_hooks.layout`; see dom.js.
 * @param {BlitzNode} node
 */
export function request_measure(node) {
	if (node.node_id !== 0) ops.push(OP.MEASURE, node.node_id);
}

/**
 * Move the host's keyboard focus: key events go to this node from now on, and
 * it gets the focus ring. The host sends no focus events for it; whoever
 * asked already knows.
 * @param {BlitzNode | null} node `null` takes the focus away from everything
 */
export function request_focus(node) {
	if (node === null) ops.push(OP.FOCUS, 0);
	else if (node.node_id !== 0) ops.push(OP.FOCUS, node.node_id);
}

/**
 * Entry points the host only uses when a module that wants them is loaded.
 *
 * `outside` is the node an event belongs to when it hit nothing the app made:
 * the area around <body>, or a page whose <body> ignores the pointer. dom.js
 * makes it <html>, as in a browser. Without it such an event is dropped.
 *
 * @type {{
 *   layout: ((width: number, height: number, boxes: number[]) => void) | null,
 *   outside: BlitzNode | null
 * }}
 */
export const host_hooks = { layout: null, outside: null };

/** @param {number} id */
export function lookup(id) {
	const entry = elements.get(id);
	if (entry === undefined) return null;
	if (has_weak_refs) return /** @type {WeakRef<BlitzNode>} */ (entry).deref() ?? null;
	return /** @type {BlitzNode} */ (entry);
}

/**
 * Things that must outlive every component. Engines that track closures
 * precisely (QuickJS) free module-level values once no live function refers to
 * them, and a FinalizationRegistry that is itself collected never calls back.
 * The entry module pins this object to `globalThis`.
 *
 * @type {{ finalizer: FinalizationRegistry<number> | null, root: BlitzNode | null }}
 */
export const keep_alive = { finalizer, root: /** @type {BlitzNode | null} */ (null) };

/** Counters for tests and leak hunting. */
export function stats() {
	let live_elements = 0;
	for (const id of elements.keys()) if (lookup(id) !== null) live_elements++;
	return { created: next_id - page.next_id, live_elements, pending_ops: ops.length };
}

/**
 * Build the mirror of the page from the host's ops, writing none: the host
 * has these nodes already. Returns the nodes, by id.
 */
function seed() {
	/** @type {Map<number, BlitzNode>} */
	const made = new Map();
	const { ops: seeded, strings: text, held } = page;
	/** @param {'element' | 'text' | 'comment'} type @param {number} id */
	const make = (type, id) => {
		let node = /** @type {BlitzNode | undefined} */ (held?.get(id));
		if (node === undefined) node = new BlitzNode(type, id);
		else {
			// A script holds this object already (`document.currentScript`):
			// it becomes the node, so what the script has is the real thing.
			Object.setPrototypeOf(node, BlitzNode.prototype);
			Object.assign(node, new BlitzNode(type, id));
		}
		finalizer?.register(node, id);
		made.set(id, node);
		return node;
	};
	for (let i = 0; i < seeded.length; ) {
		const [code, a, b, c] = seeded.slice(i, i + 4);
		switch (code) {
			case OP.CREATE_ELEMENT: {
				const node = make(ELEMENT, a);
				node.name = text[b];
				elements.set(a, has_weak_refs ? new WeakRef(node) : node);
				i += 3;
				break;
			}
			case OP.CREATE_TEXT:
			case OP.CREATE_COMMENT:
				make(code === OP.CREATE_TEXT ? TEXT : COMMENT, a).data = text[b];
				i += 3;
				break;
			case OP.SET_ATTR: {
				const node = /** @type {BlitzNode} */ (made.get(a));
				(node.attrs ??= new Map()).set(text[b], text[c]);
				if (text[b] === 'value') node.value = text[c];
				else if (text[b] === 'checked') node.checked = true;
				i += 4;
				break;
			}
			case OP.INSERT:
				link(/** @type {BlitzNode} */ (made.get(a)), /** @type {BlitzNode} */ (made.get(b)), null);
				i += 4;
				break;
			default:
				throw new Error(`sepal: the host sent a page with opcode ${code}`);
		}
	}
	return made;
}

/** @param {BlitzNode} parent @param {string} name */
const child_named = (parent, name) => {
	for (let node = parent.first; node !== null; node = node.next) if (node.name === name) return node;
	throw new Error(`sepal: the page has no <${name}>`);
};

/** `<html>`: the page's root element. The parser always makes one, with a head and a body. */
export const html = [...seed().values()].find((node) => node.parent === null && node.name === 'html') ?? child_named(new BlitzNode(ELEMENT, 0), 'html');
/** `<head>`. */
export const head = child_named(html, 'head');
/** `<body>`: where `mount` puts the app unless told otherwise. */
export const root = child_named(html, 'body');
keep_alive.root = html;

// ---------------------------------------------------------------------------
// Tree bookkeeping
// ---------------------------------------------------------------------------

/** @param {BlitzNode} node */
function unlink(node) {
	const parent = node.parent;
	if (parent === null) return;
	if (node.prev) node.prev.next = node.next;
	else parent.first = node.next;
	if (node.next) node.next.prev = node.prev;
	else parent.last = node.prev;
	node.parent = node.prev = node.next = null;
}

/**
 * @param {BlitzNode} parent
 * @param {BlitzNode} node
 * @param {BlitzNode | null} anchor
 */
function link(parent, node, anchor) {
	node.parent = parent;
	if (anchor) {
		node.next = anchor;
		node.prev = anchor.prev;
		if (anchor.prev) anchor.prev.next = node;
		else parent.first = node;
		anchor.prev = node;
	} else {
		node.prev = parent.last;
		node.next = null;
		if (parent.last) parent.last.next = node;
		else parent.first = node;
		parent.last = node;
	}
}

/**
 * @param {BlitzNode} parent
 * @param {BlitzNode} node
 * @param {BlitzNode | null} anchor
 */
function insert_one(parent, node, anchor) {
	if (node === anchor) return;
	// As in the DOM: a node cannot be put inside itself. Refusing here keeps the
	// mirror a tree, and the stack trace points at whoever asked.
	for (let /** @type {BlitzNode | null} */ ancestor = parent; ancestor !== null; ancestor = ancestor.parent) {
		if (ancestor === node) throw new Error('cannot insert a node into itself or one of its descendants');
	}
	if (anchor !== null && anchor.parent !== parent) anchor = null;

	const old_parent = node.parent;
	unlink(node);
	link(parent, node, anchor);

	// Id 0 means "not on the host": fragments, and the DOM layer's stand-ins
	// for <html> and <head>.
	if (parent.node_id !== 0) {
		// The host's insert has DOM semantics: it detaches from the old parent first.
		ops.push(OP.INSERT, parent.node_id, node.node_id, anchor ? anchor.node_id : 0);
	} else if (old_parent !== null && old_parent.node_id !== 0) {
		// Moved from the real tree to somewhere the host knows nothing about.
		ops.push(OP.REMOVE, node.node_id);
	}
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/**
 * How many listeners exist per event type, so the host can skip events nobody wants.
 * @type {Map<string, number>}
 */
const listener_counts = new Map();

/**
 * Capture listeners are kept under their own key, so the two phases of an
 * event read from separate lists. No event type contains a space.
 * @param {string} type
 */
const capture_key = (type) => `${type} capture`;

/**
 * @param {{ listeners: Map<string, Function[]> | null }} target a node, or anything else that holds listeners
 * @param {string} type
 * @param {Function} handler
 * @param {boolean} [capture] run on the way down to the target instead of on the way up
 */
export function add_listener(target, type, handler, capture = false) {
	const listeners = (target.listeners ??= new Map());
	const key = capture ? capture_key(type) : type;
	let handlers = listeners.get(key);
	if (handlers === undefined) listeners.set(key, (handlers = []));
	handlers.push(handler);

	const count = listener_counts.get(type) ?? 0;
	listener_counts.set(type, count + 1);
	if (count === 0) ops.push(OP.LISTEN, str(type));
}

/**
 * @param {{ listeners: Map<string, Function[]> | null }} target
 * @param {string} type
 * @param {Function} handler
 * @param {boolean} [capture]
 */
export function remove_listener(target, type, handler, capture = false) {
	const handlers = target.listeners?.get(capture ? capture_key(type) : type);
	if (handlers === undefined) return;
	const index = handlers.indexOf(handler);
	if (index === -1) return;
	handlers.splice(index, 1);

	const count = (listener_counts.get(type) ?? 1) - 1;
	listener_counts.set(type, count);
	if (count === 0) ops.push(OP.UNLISTEN, str(type));
}

export class BlitzEvent {
	/**
	 * @param {string} type
	 * @param {BlitzNode} target
	 * @param {boolean} bubbles
	 * @param {Record<string, any> | null} detail
	 */
	constructor(type, target, bubbles, detail) {
		if (detail) Object.assign(this, detail);
		this.type = type;
		this.target = target;
		this.currentTarget = target;
		this.bubbles = bubbles;
		this.cancelable = true;
		this.defaultPrevented = false;
		this.timeStamp = /** @type {any} */ (globalThis).performance?.now?.() ?? 0;
		this.__stopped = false;
		this.__stopped_immediately = false;
	}
	preventDefault() {
		this.defaultPrevented = true;
	}
	stopPropagation() {
		this.__stopped = true;
	}
	stopImmediatePropagation() {
		this.__stopped = true;
		this.__stopped_immediately = true;
	}
}

/**
 * Called by the host for every DOM event that has at least one listener.
 * Svelte does not delegate or bubble events for custom renderers, so we do it here.
 *
 * @param {number} target_id 0 for an event that hit nothing the app made
 * @param {string} type
 * @param {boolean} bubbles
 * @param {Record<string, any> | null} detail
 * @returns {number} DISPATCH_* flags
 */
export function dispatch(target_id, type, bubbles, detail) {
	// The host sends id 0 for an event outside everything the app made.
	const target = target_id === 0 ? host_hooks.outside : lookup(target_id);
	if (target === null) return 0;

	if (detail !== null && detail !== undefined) {
		if ('value' in detail) target.value = detail.value;
		if ('checked' in detail) target.checked = detail.checked;
		// The host reports every scroll this way, whether anyone listens or not.
		if (type === 'scroll') scroll_positions.set(target, detail);
	}

	return deliver(target, new BlitzEvent(type, target, bubbles, detail ?? null));
}

/**
 * Where an event goes after the mount target, in order. Empty by default; the
 * optional DOM layer (dom.js) puts its `document` and `window` here.
 * @type {{ listeners: Map<string, Function[]> | null }[]}
 */
export const outer_targets = [];

/**
 * Run an event's listeners: on the target, then up through its ancestors if
 * it bubbles, then on the outer targets.
 *
 * @param {BlitzNode} target
 * @param {any} event anything shaped like a `BlitzEvent`
 * @returns {number} DISPATCH_* flags
 */
export function deliver(target, event) {
	const type = event.type;
	event.target = target;

	/** @type {{ listeners: Map<string, Function[]> | null }[]} */
	const path = [];
	for (let /** @type {BlitzNode | null} */ node = target; node !== null; node = node.parent) path.push(node);
	// (An event that hit nothing starts at the first of these already.)
	for (const outer of outer_targets) if (outer !== target) path.push(outer);

	/** @param {{ listeners: Map<string, Function[]> | null }} node @param {string} key */
	const run = (node, key) => {
		const handlers = node.listeners?.get(key);
		if (handlers === undefined || handlers.length === 0) return;
		event.currentTarget = node;
		// copy: a handler may add or remove listeners
		for (const handler of handlers.slice()) {
			try {
				handler.call(node, event);
			} catch (error) {
				// `console` comes from shims.js; the ES lib does not declare it.
				/** @type {any} */ (globalThis).console.error(`Error in ${type} handler:`, error);
			}
			if (event.__stopped_immediately) break;
		}
	};

	// Down to the target, then back up: capture listeners see the event before
	// the target does, whether or not the event bubbles.
	const capture = capture_key(type);
	for (let i = path.length - 1; i >= 0 && !event.__stopped; i--) run(path[i], capture);
	for (let i = 0; i < path.length && !event.__stopped; i++) {
		run(path[i], type);
		if (!event.bubbles) break;
	}

	return (
		(event.defaultPrevented ? DISPATCH_PREVENT_DEFAULT : 0) |
		(event.__stopped ? DISPATCH_STOP_PROPAGATION : 0)
	);
}

// ---------------------------------------------------------------------------
// The renderer
// ---------------------------------------------------------------------------

/**
 * Attribute values arrive as whatever the template evaluated to.
 * @param {unknown} value
 * @returns {string | null} `null` means "remove the attribute"
 */
function normalize_attribute(value) {
	if (value === null || value === undefined || value === false) return null;
	if (value === true) return '';
	return typeof value === 'string' ? value : String(value);
}

const renderer = createRenderer({
	createFragment() {
		return new BlitzNode(FRAGMENT, 0);
	},

	createElement(name) {
		const node = create_node(ELEMENT);
		node.name = name;
		const entry = has_weak_refs ? new WeakRef(node) : node;
		elements.set(node.node_id, entry);
		if (by_tag !== null) index_tag(node, entry);
		ops.push(OP.CREATE_ELEMENT, node.node_id, str(name));
		return node;
	},

	createText(data) {
		const node = create_node(TEXT);
		node.data = data;
		ops.push(OP.CREATE_TEXT, node.node_id, str(data));
		return node;
	},

	createComment(data) {
		const node = create_node(COMMENT);
		node.data = data;
		ops.push(OP.CREATE_COMMENT, node.node_id, str(data));
		return node;
	},

	getNodeType(node) {
		return node.type;
	},

	getText(node) {
		return node.data;
	},

	setText(node, text) {
		text = text == null ? '' : String(text);
		if (node.data === text) return;
		node.data = text;
		ops.push(OP.SET_TEXT, node.node_id, str(text));
	},

	getAttribute(element, name) {
		return element.attrs?.get(name) ?? null;
	},

	hasAttribute(element, name) {
		return element.attrs?.has(name) ?? false;
	},

	setAttribute(element, name, value) {
		const normalized = normalize_attribute(value);
		if (normalized === null) {
			renderer.removeAttribute(element, name);
			return;
		}
		if (name === 'value') element.value = value;
		else if (name === 'checked') element.checked = true;

		const attrs = (element.attrs ??= new Map());
		if (attrs.get(name) === normalized) return;
		attrs.set(name, normalized);
		if (element.node_id !== 0) ops.push(OP.SET_ATTR, element.node_id, str(name), str(normalized));
	},

	removeAttribute(element, name) {
		if (name === 'checked') element.checked = false;
		if (!element.attrs?.delete(name)) return;
		if (element.node_id !== 0) ops.push(OP.REMOVE_ATTR, element.node_id, str(name));
	},

	getFirstChild(node) {
		return node.first;
	},

	getLastChild(node) {
		return node.last;
	},

	getNextSibling(node) {
		return node.next;
	},

	getParent(node) {
		return node.parent;
	},

	insert(parent, node, anchor) {
		if (node.type === FRAGMENT) {
			// Inserting a fragment inserts its children and leaves it empty.
			let child = node.first;
			while (child !== null) {
				const next = child.next;
				insert_one(parent, child, anchor);
				child = next;
			}
			return;
		}
		insert_one(parent, node, anchor);
	},

	remove(node) {
		const parent = node.parent;
		if (parent === null) return;
		unlink(node);
		if (parent.node_id !== 0) ops.push(OP.REMOVE, node.node_id);
	},

	/** @param {BlitzNode} target @param {string} type @param {Function} handler */
	addEventListener(target, type, handler) {
		add_listener(target, type, handler);
	},

	/** @param {BlitzNode} target @param {string} type @param {Function} handler */
	removeEventListener(target, type, handler) {
		remove_listener(target, type, handler);
	}
});

export default renderer;
