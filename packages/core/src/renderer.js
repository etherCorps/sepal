// The Svelte custom renderer.
//
// Svelte reads the tree synchronously (getParent, getFirstChild, getNextSibling),
// so every node is mirrored here as a small JS object with sibling pointers.
// Writes are NOT sent to the host one by one: they are appended to an op buffer
// that the host pulls once per turn with `take_batch()`. No call crosses the
// JS/Rust boundary while Svelte is rendering.

import { createRenderer } from 'svelte/renderer';
import { OP, ROOT_ID, DISPATCH_PREVENT_DEFAULT, DISPATCH_STOP_PROPAGATION } from './protocol.js';

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

let next_id = ROOT_ID + 1;

class BlitzNode {
	/**
	 * @param {'element' | 'text' | 'comment' | 'fragment'} type
	 * @param {number} id 0 for fragments, which never exist on the host
	 */
	constructor(type, id) {
		this.type = type;
		this.id = id;
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
			elements.delete(id);
			ops.push(OP.DROP, id);
		})
	: null;

/** @param {'element' | 'text' | 'comment'} type */
function create_node(type) {
	const node = new BlitzNode(type, next_id++);
	finalizer?.register(node, node.id);
	return node;
}

/** @param {number} id */
function lookup(id) {
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
export const keep_alive = { finalizer, root: null };

/** Counters for tests and leak hunting. */
export function stats() {
	let live_elements = 0;
	for (const id of elements.keys()) if (lookup(id) !== null) live_elements++;
	return { created: next_id - ROOT_ID - 1, live_elements, pending_ops: ops.length };
}

/** The mount target. The host maps it to `<body>`. Held strongly forever. */
export const root = new BlitzNode(ELEMENT, ROOT_ID);
root.name = 'body';
elements.set(ROOT_ID, has_weak_refs ? new WeakRef(root) : root);
keep_alive.root = root;

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
	if (anchor !== null && anchor.parent !== parent) anchor = null;

	const old_parent = node.parent;
	unlink(node);
	link(parent, node, anchor);

	if (parent.type !== FRAGMENT) {
		// The host's insert has DOM semantics: it detaches from the old parent first.
		ops.push(OP.INSERT, parent.id, node.id, anchor ? anchor.id : 0);
	} else if (old_parent !== null && old_parent.type !== FRAGMENT) {
		// Moved from the real tree into a fragment, which the host knows nothing about.
		ops.push(OP.REMOVE, node.id);
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

class BlitzEvent {
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
 * @param {number} target_id
 * @param {string} type
 * @param {boolean} bubbles
 * @param {Record<string, any> | null} detail
 * @returns {number} DISPATCH_* flags
 */
export function dispatch(target_id, type, bubbles, detail) {
	const target = lookup(target_id);
	if (target === null) return 0;

	if (detail !== null && detail !== undefined) {
		if ('value' in detail) target.value = detail.value;
		if ('checked' in detail) target.checked = detail.checked;
	}

	const event = new BlitzEvent(type, target, bubbles, detail ?? null);

	/** @type {BlitzNode | null} */
	let node = target;
	while (node !== null) {
		const handlers = node.listeners?.get(type);
		if (handlers !== undefined && handlers.length > 0) {
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
		}
		if (!bubbles || event.__stopped) break;
		node = node.parent;
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
		elements.set(node.id, has_weak_refs ? new WeakRef(node) : node);
		ops.push(OP.CREATE_ELEMENT, node.id, str(name));
		return node;
	},

	createText(data) {
		const node = create_node(TEXT);
		node.data = data;
		ops.push(OP.CREATE_TEXT, node.id, str(data));
		return node;
	},

	createComment(data) {
		const node = create_node(COMMENT);
		node.data = data;
		ops.push(OP.CREATE_COMMENT, node.id, str(data));
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
		ops.push(OP.SET_TEXT, node.id, str(text));
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
		ops.push(OP.SET_ATTR, element.id, str(name), str(normalized));
	},

	removeAttribute(element, name) {
		if (name === 'checked') element.checked = false;
		if (!element.attrs?.delete(name)) return;
		ops.push(OP.REMOVE_ATTR, element.id, str(name));
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
		if (parent.type !== FRAGMENT) ops.push(OP.REMOVE, node.id);
	},

	addEventListener(target, type, handler) {
		const listeners = (target.listeners ??= new Map());
		let handlers = listeners.get(type);
		if (handlers === undefined) listeners.set(type, (handlers = []));
		handlers.push(handler);

		const count = listener_counts.get(type) ?? 0;
		listener_counts.set(type, count + 1);
		if (count === 0) ops.push(OP.LISTEN, str(type));
	},

	removeEventListener(target, type, handler) {
		const handlers = target.listeners?.get(type);
		if (handlers === undefined) return;
		const index = handlers.indexOf(handler);
		if (index === -1) return;
		handlers.splice(index, 1);

		const count = (listener_counts.get(type) ?? 1) - 1;
		listener_counts.set(type, count);
		if (count === 0) ops.push(OP.UNLISTEN, str(type));
	}
});

export default renderer;
