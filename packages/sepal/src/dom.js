// A small browser for libraries that expect one.
//
// Sepal itself needs no DOM globals, and an app that only uses its own
// components should not load this file. Component libraries written for the
// web do reach for `window`, `document`, `element.addEventListener` and so
// on; `dom: true` in sepal.config makes the preset load this module first, and
// builds the app as a browser bundle, so those libraries find what they need.
//
// What is here is what such libraries were seen to use, not the DOM. Things
// with no real answer in this host give a harmless one: an element measures
// as an empty rectangle, computed styles are blank, observers never fire.
//
// The tree is the renderer's own. An element here is a `BlitzNode`, and every
// write goes through the renderer into the same op buffer as Svelte's.

import { host_call } from './host-call.js';
import { TextDecoder, TextEncoder } from './text.js';
import { URL, URLSearchParams } from './url.js';
import renderer, { BlitzEvent, BlitzNode, add_listener, deliver, host_hooks, lookup, outer_targets, remove_listener, request_focus, request_measure, root, scroll_state } from './renderer.js';

const g = /** @type {any} */ (globalThis);
const proto = /** @type {any} */ (BlitzNode.prototype);

/** @param {object} target @param {Record<string, PropertyDescriptor & ThisType<any>>} properties */
function define(target, properties) {
	for (const name of Object.keys(properties)) {
		Object.defineProperty(target, name, { configurable: true, ...properties[name] });
	}
}

// ---------------------------------------------------------------------------
// Selectors: tag, #id, .class, [attr], [attr=value] and its variants, :not(),
// a few state pseudo-classes, descendant and child combinators, comma lists.
// ---------------------------------------------------------------------------

/**
 * @typedef {{ tag: string | null, id: string | null, classes: string[],
 *   attrs: { name: string, op: string | null, value: string }[],
 *   nots: Compound[][][], pseudos: string[] }} Compound
 */

const SIMPLE =
	/^(?:([a-zA-Z][\w-]*|\*)|#((?:\\.|[\w-])+)|\.((?:\\.|[\w-])+)|\[\s*([\w:-]+)\s*(?:([~|^$*]?=)\s*(?:"([^"]*)"|'([^']*)'|([^\]\s]+))\s*(?:[is]\s*)?)?\]|:not\(((?:[^()]|\([^()]*\))*)\)|:([\w-]+)(?:\([^)]*\))?)/;

/** @type {Map<string, { compounds: Compound[], combinators: string[] }[]>} */
const parsed = new Map();

/** @param {string} selector */
function parse_selector(selector) {
	let list = parsed.get(selector);
	if (list !== undefined) return list;
	list = [];
	let rest = selector.trim();
	let complex = { compounds: /** @type {Compound[]} */ ([]), combinators: /** @type {string[]} */ ([]) };
	/** @type {Compound | null} */
	let compound = null;

	while (rest.length > 0) {
		const space = /^\s*([>,+~]?)\s*/.exec(rest);
		if (space !== null && space[0].length > 0 && (space[1] !== '' || compound !== null)) {
			const combinator = space[1];
			rest = rest.slice(space[0].length);
			if (rest.length === 0) break;
			if (compound !== null) complex.compounds.push(compound);
			compound = null;
			if (combinator === ',') {
				list.push(complex);
				complex = { compounds: [], combinators: [] };
			} else {
				complex.combinators.push(combinator || ' ');
			}
			continue;
		}
		const part = SIMPLE.exec(rest);
		if (part === null) throw new Error(`sepal: cannot parse the selector ${selector}`);
		rest = rest.slice(part[0].length);
		compound ??= { tag: null, id: null, classes: [], attrs: [], nots: [], pseudos: [] };
		const unescape = (/** @type {string} */ text) => text.replace(/\\(.)/g, '$1');
		if (part[1] !== undefined) compound.tag = part[1] === '*' ? null : part[1].toLowerCase();
		else if (part[2] !== undefined) compound.id = unescape(part[2]);
		else if (part[3] !== undefined) compound.classes.push(unescape(part[3]));
		else if (part[4] !== undefined) compound.attrs.push({ name: part[4], op: part[5] ?? null, value: part[6] ?? part[7] ?? part[8] ?? '' });
		else if (part[9] !== undefined) compound.nots.push(parse_selector(part[9]).map((c) => [c.compounds[c.compounds.length - 1]]));
		else compound.pseudos.push(part[10]);
	}
	if (compound !== null) complex.compounds.push(compound);
	if (complex.compounds.length > 0) list.push(complex);
	parsed.set(selector, list);
	return list;
}

/** @param {any} node @param {Compound} compound */
function matches_compound(node, compound) {
	if (node.type !== 'element') return false;
	if (compound.tag !== null && node.name.toLowerCase() !== compound.tag) return false;
	const attr = (/** @type {string} */ name) => node.attrs?.get(name) ?? null;
	if (compound.id !== null && attr('id') !== compound.id) return false;
	if (compound.classes.length > 0) {
		const classes = (attr('class') ?? '').split(/\s+/);
		if (!compound.classes.every((name) => classes.includes(name))) return false;
	}
	for (const { name, op, value } of compound.attrs) {
		const actual = attr(name);
		if (actual === null) return false;
		if (op === '=' && actual !== value) return false;
		if (op === '~=' && !actual.split(/\s+/).includes(value)) return false;
		if (op === '^=' && !actual.startsWith(value)) return false;
		if (op === '$=' && !actual.endsWith(value)) return false;
		if (op === '*=' && !actual.includes(value)) return false;
		if (op === '|=' && actual !== value && !actual.startsWith(value + '-')) return false;
	}
	for (const alternatives of compound.nots) {
		if (alternatives.some(([inner]) => matches_compound(node, inner))) return false;
	}
	for (const pseudo of compound.pseudos) {
		if (pseudo === 'disabled' ? attr('disabled') === null : pseudo === 'enabled' ? attr('disabled') !== null : pseudo === 'checked' ? !node.checked && attr('checked') === null : pseudo === 'focus' ? active !== node : pseudo === 'first-child' ? node.prev !== null && previous_element(node) !== null : false) {
			return false;
		}
	}
	return true;
}

/** @param {any} node */
const previous_element = (node) => {
	for (let other = node.prev; other !== null; other = other.prev) if (other.type === 'element') return other;
	return null;
};

/** @param {any} node @param {{ compounds: Compound[], combinators: string[] }} complex @param {number} [index] @returns {boolean} */
function matches_complex(node, complex, index = complex.compounds.length - 1) {
	if (!matches_compound(node, complex.compounds[index])) return false;
	if (index === 0) return true;
	const combinator = complex.combinators[index - 1];
	if (combinator === '>') return node.parent !== null && matches_complex(node.parent, complex, index - 1);
	if (combinator === '+') return previous_element(node) !== null && matches_complex(previous_element(node), complex, index - 1);
	if (combinator === '~') {
		for (let other = previous_element(node); other !== null; other = previous_element(other)) {
			if (matches_complex(other, complex, index - 1)) return true;
		}
		return false;
	}
	for (let ancestor = node.parent; ancestor !== null; ancestor = ancestor.parent) {
		if (matches_complex(ancestor, complex, index - 1)) return true;
	}
	return false;
}

/** @param {any} node @param {string} selector */
const matches = (node, selector) => parse_selector(selector).some((complex) => matches_complex(node, complex));

/** Every element below `scope`, in document order. @param {any} scope @param {(node: any) => boolean | void} visit returns true to stop */
function walk(scope, visit) {
	for (let node = scope.first; node !== null; node = node.next) {
		if (node.type === 'element' && visit(node) === true) return true;
		if (walk(node, visit) === true) return true;
	}
	return false;
}

/** @param {any} scope @param {string} selector */
function query_all(scope, selector) {
	/** @type {any[]} */
	const found = [];
	walk(scope, (node) => {
		if (matches(node, selector)) found.push(node);
	});
	return found;
}

/** @param {any} scope @param {string} selector */
function query(scope, selector) {
	let found = null;
	walk(scope, (node) => {
		if (!matches(node, selector)) return false;
		found = node;
		return true;
	});
	return found;
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

class Event {
	/** @param {string} type @param {Record<string, any>} [init] */
	constructor(type, init = {}) {
		Object.assign(this, init);
		this.type = type;
		this.bubbles = init.bubbles ?? false;
		this.cancelable = init.cancelable ?? false;
		this.composed = init.composed ?? false;
		this.defaultPrevented = false;
		/** @type {any} */
		this.target = null;
		/** @type {any} */
		this.currentTarget = null;
		this.timeStamp = g.performance?.now?.() ?? 0;
		this.isTrusted = false;
		this.__stopped = false;
		this.__stopped_immediately = false;
	}
	preventDefault() {
		if (this.cancelable) this.defaultPrevented = true;
	}
	stopPropagation() {
		this.__stopped = true;
	}
	stopImmediatePropagation() {
		this.__stopped = true;
		this.__stopped_immediately = true;
	}
	composedPath() {
		return composed_path(this.target);
	}
	get cancelBubble() {
		return this.__stopped;
	}
	// Events from the host are `BlitzEvent`s; to a library they are Events.
	static [Symbol.hasInstance](/** @type {any} */ value) {
		return value instanceof BlitzEvent || Function.prototype[Symbol.hasInstance].call(this, value);
	}
}

/** @param {any} target */
function composed_path(target) {
	const path = [];
	for (let node = target; node; node = node.parent) path.push(node);
	return [...path, document, g];
}

class CustomEvent extends Event {
	/** @param {string} type @param {Record<string, any>} [init] */
	constructor(type, init = {}) {
		super(type, init);
		this.detail = init.detail ?? null;
	}
}

/**
 * An event class that host events of the right kind also count as.
 * @param {RegExp} types
 */
function event_class(types) {
	return class extends Event {
		static [Symbol.hasInstance](/** @type {any} */ value) {
			return (value instanceof BlitzEvent && types.test(value.type)) || Function.prototype[Symbol.hasInstance].call(this, value);
		}
	};
}

const MouseEvent = event_class(/^(click|dblclick|contextmenu|mouse|pointer)/);
const PointerEvent = event_class(/^pointer/);
const KeyboardEvent = event_class(/^key/);
const FocusEvent = event_class(/^(focus|blur)/);
const InputEvent = event_class(/^(input|beforeinput)$/);

// What host events lack next to browser ones.
define(BlitzEvent.prototype, {
	composedPath: {
		value() {
			return composed_path(this.target);
		}
	},
	cancelBubble: {
		get() {
			return this.__stopped;
		}
	},
	isTrusted: { value: true },
	pointerType: { value: 'mouse', writable: true },
	pointerId: { value: 1, writable: true },
	isPrimary: { value: true, writable: true },
	detail: { value: 1, writable: true },
	view: { get: () => g }
});

/** A standalone event target, for `class Store extends EventTarget` and the like. */
class EventTarget {
	constructor() {
		/** @type {Map<string, Function[]> | null} */
		this.listeners = null;
	}
	/** @param {string} type @param {any} handler @param {any} [options] */
	addEventListener(type, handler, options) {
		listen(this, type, handler, options);
	}
	/** @param {string} type @param {any} handler @param {any} [options] */
	removeEventListener(type, handler, options) {
		unlisten(this, type, handler, options);
	}
	/** @param {any} event */
	dispatchEvent(event) {
		event.target = event.currentTarget = this;
		for (const handler of (this.listeners?.get(event.type) ?? []).slice()) handler.call(this, event);
		return !event.defaultPrevented;
	}
}

/** Handlers as registered, so `removeEventListener` finds the wrapper made for `once` or `handleEvent`. */
const wrappers = new WeakMap();

/** `addEventListener`'s third argument is `true`, or an object with `capture`. @param {any} options */
const is_capture = (options) => options === true || (typeof options === 'object' && options !== null && options.capture === true);

/** @param {any} target @param {string} type @param {any} handler @param {any} [options] */
function listen(target, type, handler, options) {
	if (handler == null) return;
	const capture = is_capture(options);
	let actual = typeof handler === 'function' ? handler : (/** @type {any} */ event) => handler.handleEvent(event);
	if (options && typeof options === 'object' && options.once) {
		const inner = actual;
		actual = /** @this {any} @param {any} event */ function (event) {
			unlisten(target, type, handler, capture);
			return inner.call(this, event);
		};
	}
	if (actual !== handler) {
		let by_type = wrappers.get(handler);
		if (by_type === undefined) wrappers.set(handler, (by_type = new Map()));
		by_type.set(capture ? `${type} capture` : type, actual);
	}
	add_listener(target, type, actual, capture);
	const signal = options && typeof options === 'object' ? options.signal : null;
	if (signal) signal.addEventListener('abort', () => unlisten(target, type, handler, capture));
}

/** @param {any} target @param {string} type @param {any} handler @param {any} [options] */
function unlisten(target, type, handler, options) {
	if (handler == null) return;
	const capture = is_capture(options);
	remove_listener(target, type, wrappers.get(handler)?.get(capture ? `${type} capture` : type) ?? handler, capture);
}

class AbortSignal extends EventTarget {
	constructor() {
		super();
		this.aborted = false;
		/** @type {any} */
		this.reason = undefined;
	}
	throwIfAborted() {
		if (this.aborted) throw this.reason;
	}
}

class AbortController {
	constructor() {
		this.signal = new AbortSignal();
	}
	/** @param {any} [reason] */
	abort(reason) {
		if (this.signal.aborted) return;
		this.signal.aborted = true;
		this.signal.reason = reason ?? new Error('aborted');
		this.signal.dispatchEvent(new Event('abort'));
	}
}

// ---------------------------------------------------------------------------
// Elements
// ---------------------------------------------------------------------------

/** The element with focus, as far as JavaScript knows. @type {any} */
let active = null;

/** `element.style`: reads and writes the `style` attribute. @param {any} node */
function style_of(node) {
	const read = () => {
		/** @type {Map<string, string>} */
		const declarations = new Map();
		for (const declaration of (node.attrs?.get('style') ?? '').split(';')) {
			const colon = declaration.indexOf(':');
			if (colon > 0) declarations.set(declaration.slice(0, colon).trim(), declaration.slice(colon + 1).trim());
		}
		return declarations;
	};
	const write = (/** @type {Map<string, string>} */ declarations) => {
		const text = [...declarations].map(([name, value]) => `${name}: ${value}`).join('; ');
		if (text === '') renderer.removeAttribute(node, 'style');
		else renderer.setAttribute(node, 'style', text);
	};
	const kebab = (/** @type {string} */ name) => (name.startsWith('--') ? name : name.replace(/[A-Z]/g, (c) => '-' + c.toLowerCase()));
	const set = (/** @type {string} */ name, /** @type {any} */ value) => {
		const declarations = read();
		if (value == null || value === '') declarations.delete(name);
		else declarations.set(name, String(value));
		write(declarations);
	};
	return new Proxy(
		{},
		{
			get(_, name) {
				if (name === 'setProperty') return set;
				if (name === 'removeProperty') return (/** @type {string} */ n) => set(n, null);
				if (name === 'getPropertyValue') return (/** @type {string} */ n) => read().get(n) ?? '';
				if (name === 'getPropertyPriority') return () => '';
				if (name === 'cssText') return node.attrs?.get('style') ?? '';
				if (typeof name !== 'string') return undefined;
				return read().get(kebab(name)) ?? '';
			},
			set(_, name, value) {
				if (name === 'cssText') renderer.setAttribute(node, 'style', String(value));
				else if (typeof name === 'string') set(kebab(name), value);
				return true;
			}
		}
	);
}

/** @param {any} node */
function text_of(node) {
	if (node.type === 'text') return node.data;
	let text = '';
	for (let child = node.first; child !== null; child = child.next) text += child.type === 'comment' ? '' : text_of(child);
	return text;
}

const NODE_TYPES = { element: 1, text: 3, comment: 8, fragment: 11 };
// ---------------------------------------------------------------------------
// Layout
//
// Layout happens in the host. A measurement gets there in two ways.
//
// Asked: `__sb_host.measure(id)` is a forced layout, as in a browser. The host
// applies what the app has written so far, lays the page out and returns the
// node's box. Code that changes the page and measures it on the next line
// depends on this.
//
// Told: the first time a node is measured it is also marked with one op, and
// from then on the host sends its box after every layout that moved or
// resized it. That is what `ResizeObserver` runs on, and it is the answer
// when the host cannot lay out: inside an event handler, where Blitz is in
// the middle of the document, and before the window has a size.
//
// Boxes are in viewport coordinates and do not include CSS transforms.
// ---------------------------------------------------------------------------

class DOMRect {
	constructor(x = 0, y = 0, width = 0, height = 0) {
		Object.assign(this, { x, y, width, height, top: y, left: x, right: x + width, bottom: y + height });
	}
	toJSON() {
		return { ...this };
	}
}

/** `[x, y, width, height]` per measured node. @type {WeakMap<object, number[]>} */
const boxes = new WeakMap();
const NO_BOX = [0, 0, 0, 0];
/** The window's size in CSS pixels. A guess until the host has laid the page out once. */
const viewport = { width: 1024, height: 768 };

/** The box the host last reported. @param {any} node @returns {number[]} */
function reported_box(node) {
	let known = boxes.get(node);
	if (known === undefined) {
		boxes.set(node, (known = NO_BOX));
		request_measure(node);
	}
	return known;
}

/**
 * A forced layout: `[x, y, width, height, animating, clientWidth,
 * clientHeight, scrollWidth, scrollHeight]` as of now, or nothing when the
 * host cannot lay out at this moment.
 * @param {any} node
 * @returns {number[] | null | undefined}
 */
const forced_layout = (node) => (node.node_id === 0 ? null : g.__sb_host.measure?.(node.node_id));

/**
 * The box as of now if the host can lay out now, with what follows it in a
 * forced layout; otherwise the last box reported.
 * @param {any} node @returns {number[]}
 */
function box(node) {
	const reported = reported_box(node);
	const fresh = forced_layout(node);
	if (!fresh) return reported;
	boxes.set(node, fresh.slice(0, 4));
	return fresh;
}

/**
 * What `getAnimations()` returns for a node with CSS animations or
 * transitions running: one object that stands for all of them, finished when
 * the last one is. Enough to wait on, which is what it is used for.
 * @param {number} seconds until the last one ends; infinite if one repeats forever
 */
function running_animation(seconds) {
	/** @type {any} */
	const animation = { playState: 'running', pending: false, cancel() {}, finish() {}, pause() {}, play() {} };
	animation.finished = new Promise((resolve) => {
		if (seconds === Infinity) return;
		g.setTimeout(() => {
			animation.playState = 'finished';
			resolve(animation);
		}, seconds * 1000);
	});
	animation.ready = Promise.resolve(animation);
	return animation;
}

/** @type {Set<ResizeObserver>} the observers that are watching something */
const resize_observers = new Set();

class ResizeObserver {
	/** @param {(entries: any[], observer: ResizeObserver) => void} callback */
	constructor(callback) {
		this.callback = callback;
		/** The size each target had when it was last reported. @type {Map<any, number[]>} */
		this.targets = new Map();
	}
	/** @param {any} target */
	observe(target) {
		if (!target || this.targets.has(target)) return;
		this.targets.set(target, [0, 0]);
		resize_observers.add(this);
		reported_box(target);
		// A box that is already known is not sent again, so report it from here.
		g.queueMicrotask(() => this.deliver());
	}
	/** @param {any} target */
	unobserve(target) {
		this.targets.delete(target);
		if (this.targets.size === 0) resize_observers.delete(this);
	}
	disconnect() {
		this.targets.clear();
		resize_observers.delete(this);
	}
	/** Report the targets whose size is not what was last reported. */
	deliver() {
		const entries = [];
		for (const [target, last] of this.targets) {
			// Called right after a layout: what was reported is current.
			const [, , width, height] = reported_box(target);
			if (last[0] === width && last[1] === height) continue;
			this.targets.set(target, [width, height]);
			const size = [{ inlineSize: width, blockSize: height }];
			entries.push({ target, contentRect: new DOMRect(0, 0, width, height), borderBoxSize: size, contentBoxSize: size, devicePixelContentBoxSize: size });
		}
		if (entries.length === 0) return;
		try {
			this.callback(entries, this);
		} catch (error) {
			g.console.error('Error in ResizeObserver callback:', error);
		}
	}
}

// The host's entry point: called after a layout that changed something the app measures.
host_hooks.layout = (width, height, changed) => {
	for (let i = 0; i < changed.length; i += 5) {
		const node = lookup(changed[i]);
		if (node !== null) boxes.set(node, [changed[i + 1], changed[i + 2], changed[i + 3], changed[i + 4]]);
	}
	const resized = width !== viewport.width || height !== viewport.height;
	viewport.width = width;
	viewport.height = height;
	for (const observer of [...resize_observers]) observer.deliver();
	if (resized) g.dispatchEvent(new Event('resize'));
};

/** An attribute that is also a property: `el.id`, `el.tabIndex`. @param {string} attribute */
const reflected = (attribute) => ({
	/** @this {any} */
	get() {
		return this.attrs?.get(attribute) ?? '';
	},
	/** @this {any} @param {any} value */
	set(value) {
		renderer.setAttribute(this, attribute, String(value));
	}
});

/** A boolean attribute that is also a property: `el.hidden`, `el.disabled`. @param {string} attribute */
const reflected_flag = (attribute) => ({
	/** @this {any} */
	get() {
		return this.attrs?.has(attribute) ?? false;
	},
	/** @this {any} @param {any} value */
	set(value) {
		if (value) renderer.setAttribute(this, attribute, '');
		else renderer.removeAttribute(this, attribute);
	}
});

define(proto, {
	nodeType: {
		get() {
			return NODE_TYPES[/** @type {keyof typeof NODE_TYPES} */ (this.type)];
		}
	},
	tagName: {
		get() {
			return this.name.toUpperCase();
		}
	},
	localName: {
		get() {
			return this.name.toLowerCase();
		}
	},
	ownerDocument: { get: () => document },
	isConnected: {
		get() {
			for (let node = this; node !== null; node = node.parent) if (node === root) return true;
			return false;
		}
	},
	parentElement: {
		get() {
			return this.parent !== null && this.parent.type === 'element' ? this.parent : null;
		}
	},
	firstChild: {
		get() {
			return this.first;
		}
	},
	lastChild: {
		get() {
			return this.last;
		}
	},
	nextSibling: {
		get() {
			return this.next;
		}
	},
	previousSibling: {
		get() {
			return this.prev;
		}
	},
	childNodes: {
		get() {
			const nodes = [];
			for (let node = this.first; node !== null; node = node.next) nodes.push(node);
			return nodes;
		}
	},
	children: {
		get() {
			return this.childNodes.filter((/** @type {any} */ node) => node.type === 'element');
		}
	},
	firstElementChild: {
		get() {
			return this.children[0] ?? null;
		}
	},
	nextElementSibling: {
		get() {
			for (let node = this.next; node !== null; node = node.next) if (node.type === 'element') return node;
			return null;
		}
	},
	previousElementSibling: {
		get() {
			return previous_element(this);
		}
	},
	textContent: {
		get() {
			return text_of(this);
		},
		set(/** @type {any} */ value) {
			if (this.type !== 'element') return renderer.setText(this, String(value));
			while (this.first !== null) renderer.remove(this.first);
			renderer.insert(this, renderer.createText(String(value)), null);
		}
	},
	id: reflected('id'),
	className: reflected('class'),
	title: reflected('title'),
	role: reflected('role'),
	// For a link, the address it leads to, in full: routers read this and not
	// the attribute.
	href: {
		/** @this {any} */
		get() {
			const written = this.attrs?.get('href');
			if (written === undefined) return '';
			return URL.parse(written, address.href)?.href ?? written;
		},
		/** @this {any} @param {any} value */
		set(value) {
			renderer.setAttribute(this, 'href', String(value));
		}
	},
	hidden: reflected_flag('hidden'),
	disabled: reflected_flag('disabled'),
	tabIndex: {
		get() {
			const value = this.attrs?.get('tabindex');
			return value == null ? (/^(a|button|input|select|textarea)$/i.test(this.name) ? 0 : -1) : Number(value);
		},
		set(/** @type {any} */ value) {
			renderer.setAttribute(this, 'tabindex', String(value));
		}
	},
	dataset: {
		get() {
			const node = this;
			const attribute = (/** @type {string} */ name) => 'data-' + name.replace(/[A-Z]/g, (c) => '-' + c.toLowerCase());
			return new Proxy(
				{},
				{
					get: (_, name) => (typeof name === 'string' ? (node.attrs?.get(attribute(name)) ?? undefined) : undefined),
					set: (_, name, value) => (renderer.setAttribute(node, attribute(String(name)), String(value)), true),
					has: (_, name) => typeof name === 'string' && (node.attrs?.has(attribute(name)) ?? false),
					deleteProperty: (_, name) => (renderer.removeAttribute(node, attribute(String(name))), true)
				}
			);
		}
	},
	style: {
		get() {
			return style_of(this);
		}
	},
	classList: {
		get() {
			const node = this;
			const read = () => (node.attrs?.get('class') ?? '').split(/\s+/).filter(Boolean);
			const write = (/** @type {string[]} */ names) => renderer.setAttribute(node, 'class', names.join(' '));
			return {
				contains: (/** @type {string} */ name) => read().includes(name),
				add: (/** @type {string[]} */ ...names) => write([...new Set([...read(), ...names])]),
				remove: (/** @type {string[]} */ ...names) => write(read().filter((/** @type {string} */ name) => !names.includes(name))),
				toggle(/** @type {string} */ name, /** @type {boolean} */ force) {
					const on = force ?? !read().includes(name);
					if (on) this.add(name);
					else this.remove(name);
					return on;
				}
			};
		}
	},
	// From the node's box (see "Layout"). The client and scroll sizes come
	// with a forced layout; when there can be none, from the last `scroll`
	// event, or the box. `scrollTop` and the scroll methods are the renderer's.
	...Object.fromEntries(
		/** @type {const} */ ([
			['offsetLeft', 0],
			['offsetTop', 1],
			['offsetWidth', 2],
			['offsetHeight', 3],
			['clientWidth', 5, 2],
			['clientHeight', 6, 3],
			['scrollWidth', 7, 2],
			['scrollHeight', 8, 3]
		]).map(([name, index, fallback]) => [
			name,
			{
				/** @this {any} */
				get() {
					const known = box(this);
					return Math.round(known[index] ?? scroll_state(this)?.[name] ?? known[fallback ?? index]);
				}
			}
		])
	),
	offsetParent: { value: null }
});

Object.assign(proto, /** @type {Record<string, any> & ThisType<any>} */ ({
	/** @param {string} name @param {any} value */
	setAttribute(name, value) {
		renderer.setAttribute(this, name, String(value));
	},
	/** @param {string} name */
	removeAttribute(name) {
		renderer.removeAttribute(this, name);
	},
	/** @param {string} name */
	hasAttribute(name) {
		return this.attrs?.has(name) ?? false;
	},
	getAttributeNames() {
		return [...(this.attrs?.keys() ?? [])];
	},
	/** @param {any} child */
	appendChild(child) {
		renderer.insert(this, child, null);
		return child;
	},
	/** @param {any[]} children */
	append(...children) {
		for (const child of children) this.appendChild(typeof child === 'string' ? renderer.createText(child) : child);
	},
	/** @param {any} child @param {any} before */
	insertBefore(child, before) {
		renderer.insert(this, child, before ?? null);
		return child;
	},
	/** @param {any} child */
	removeChild(child) {
		renderer.remove(child);
		return child;
	},
	remove() {
		renderer.remove(this);
	},
	/** @param {any} other */
	contains(other) {
		for (let node = other; node; node = node.parent) if (node === this) return true;
		return false;
	},
	/** @param {any} other */
	compareDocumentPosition(other) {
		if (other === this) return 0;
		if (this.contains(other)) return 16 | 4;
		if (other.contains(this)) return 8 | 2;
		/** @type {any[]} */
		const order = [];
		walk(root, (node) => void order.push(node));
		return order.indexOf(this) < order.indexOf(other) ? 4 : 2;
	},
	getRootNode() {
		return document;
	},
	/** @param {string} selector */
	matches(selector) {
		return matches(this, selector);
	},
	/** @param {string} selector */
	closest(selector) {
		for (let node = this; node !== null; node = node.parent) if (matches(node, selector)) return node;
		return null;
	},
	/** @param {string} selector */
	querySelector(selector) {
		return query(this, selector);
	},
	/** @param {string} selector */
	querySelectorAll(selector) {
		return query_all(this, selector);
	},
	getBoundingClientRect() {
		return new DOMRect(...box(this));
	},
	getClientRects() {
		return [new DOMRect(...box(this))];
	},
	// The host knows what is animating. It is asked along with a forced
	// layout, so an animation that the last change started is counted.
	getAnimations() {
		const seconds = forced_layout(this)?.[4] ?? 0;
		return seconds > 0 ? [running_animation(seconds)] : [];
	},
	setPointerCapture() {},
	releasePointerCapture() {},
	hasPointerCapture: () => false,
	/** @param {string} type @param {any} handler @param {any} [options] */
	addEventListener(type, handler, options) {
		listen(this, type, handler, options);
	},
	/** @param {string} type @param {any} handler @param {any} [options] */
	removeEventListener(type, handler, options) {
		unlisten(this, type, handler, options);
	},
	/** @param {any} event */
	dispatchEvent(event) {
		deliver(this, event);
		return !event.defaultPrevented;
	},
	click() {
		this.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, button: 0 }));
	},
	// The host owns the keyboard focus: a click or Tab moves it and the host
	// says so with focus events. Moving it from here is the other direction:
	// the host is told with an op, and the events are delivered right away,
	// because code that calls `focus()` reads `activeElement` on the next line.
	focus() {
		if (active === this || !is_focusable(this)) return;
		const previous = active;
		active = this;
		request_focus(this);
		if (previous !== null) {
			deliver(previous, new FocusEvent('blur', { relatedTarget: this }));
			deliver(previous, new FocusEvent('focusout', { bubbles: true, relatedTarget: this }));
		}
		deliver(this, new FocusEvent('focus', { relatedTarget: previous }));
		deliver(this, new FocusEvent('focusin', { bubbles: true, relatedTarget: previous }));
	},
	blur() {
		if (active !== this) return;
		active = null;
		request_focus(null);
		deliver(this, new FocusEvent('blur', {}));
		deliver(this, new FocusEvent('focusout', { bubbles: true }));
	}
}));

/**
 * What `focus()` works on: controls and links, and anything with `tabindex`.
 * On anything else it does nothing, and callers check `activeElement` to see.
 * @param {any} node
 */
function is_focusable(node) {
	if (node.type !== 'element' || node.attrs?.has('disabled')) return false;
	if (node.attrs?.has('tabindex') || node.attrs?.has('contenteditable')) return true;
	return /^(button|input|select|textarea|summary)$/i.test(node.name) || (/^(a|area)$/i.test(node.name) && node.attrs?.has('href'));
}

/** A DOM class that elements with one of these tag names count as. @param {((node: any) => boolean)} test */
const node_class = (test) => {
	const stand_in = function () {
		throw new TypeError('Illegal constructor');
	};
	// Libraries borrow methods from here: `Element.prototype.matches.call(node, selector)`.
	stand_in.prototype = BlitzNode.prototype;
	Object.defineProperty(stand_in, Symbol.hasInstance, { value: (/** @type {any} */ value) => value instanceof BlitzNode && test(value) });
	return stand_in;
};

const is_element = (/** @type {any} */ node) => node.type === 'element';
const SVG_TAGS = /^(svg|g|path|circle|ellipse|line|polyline|polygon|rect|defs|use|symbol|text|tspan|mask|pattern|marker|filter|image)$/;
/** @param {string} tags */
const tag_class = (tags) => node_class((node) => is_element(node) && new RegExp(`^(${tags})$`, 'i').test(node.name));

// ---------------------------------------------------------------------------
// document and window
// ---------------------------------------------------------------------------

/** Stands in for <html> and <head>: something to hold attributes and styles that go nowhere. */
const detached = () => new BlitzNode('element', 0);

// ---------------------------------------------------------------------------
// The address, and the history of it
//
// An app is not at a URL: nothing was fetched from anywhere. But routers are
// written against `location` and `history`, so there is an address, starting
// at `http://app.localhost/`, that only ever changes through `history` or
// `location` here. Nothing is loaded when it does; whoever listens to
// `popstate` decides what that means.
// ---------------------------------------------------------------------------

const address = new URL('http://app.localhost/');
/** @type {{ href: string, state: any }[]} */
const visited = [{ href: address.href, state: null }];
let visiting = 0;

class PopStateEvent extends Event {
	/** @param {string} type @param {{ state?: any }} [init] */
	constructor(type, init = {}) {
		super(type, init);
		this.state = init.state ?? null;
	}
}

/** Go to another entry and say so, as a browser does after `back()`. @param {number} index */
function travel(index) {
	if (index < 0 || index >= visited.length || index === visiting) return;
	const from = address.href;
	visiting = index;
	address.href = visited[index].href;
	g.dispatchEvent(new PopStateEvent('popstate', { state: visited[index].state }));
	if (from.split('#')[0] === address.href.split('#')[0] && from !== address.href) {
		g.dispatchEvent(Object.assign(new Event('hashchange'), { oldURL: from, newURL: address.href }));
	}
}

/** @param {any} state @param {string | URL | null | undefined} url @param {boolean} push */
function record(state, url, push) {
	if (url !== null && url !== undefined) {
		const next = new URL(String(url), address.href);
		if (next.origin !== address.origin) throw new Error(`Cannot go from ${address.origin} to ${next.origin} with history`);
		address.href = next.href;
	}
	if (push) visited.splice(++visiting, Infinity, { href: address.href, state });
	else visited[visiting] = { href: address.href, state };
}

const history = {
	scrollRestoration: 'auto',
	get length() {
		return visited.length;
	},
	get state() {
		return visited[visiting].state;
	},
	/** @param {any} state @param {string} _title @param {string | URL | null} [url] */
	pushState: (state, _title, url) => record(state, url, true),
	/** @param {any} state @param {string} _title @param {string | URL | null} [url] */
	replaceState: (state, _title, url) => record(state, url, false),
	// In a browser these take effect in a later task.
	/** @param {number} [delta] */
	go: (delta = 0) => void g.setTimeout(() => travel(visiting + delta), 0),
	back: () => history.go(-1),
	forward: () => history.go(1)
};

/** `location`: the address, readable by its parts, and assignable. */
const location = {
	/** @param {string | URL} url */
	assign(url) {
		record(null, url, true);
		g.dispatchEvent(new PopStateEvent('popstate', { state: null }));
	},
	/** @param {string | URL} url */
	replace(url) {
		record(null, url, false);
		g.dispatchEvent(new PopStateEvent('popstate', { state: null }));
	},
	reload() {},
	toString: () => address.href
};
for (const part of /** @type {const} */ (['href', 'origin', 'protocol', 'host', 'hostname', 'port', 'pathname', 'search', 'hash'])) {
	Object.defineProperty(location, part, {
		enumerable: true,
		get: () => address[part],
		set(value) {
			if (part === 'origin') return;
			const next = new URL(address.href);
			/** @type {any} */ (next)[part] = value;
			location.assign(next);
		}
	});
}

/** `sessionStorage`: lasts as long as the app runs, which is what a session is here. */
function memory_storage() {
	/** @type {Map<string, string>} */
	const items = new Map();
	return new Proxy(
		{
			/** @param {string} key */
			getItem: (key) => items.get(String(key)) ?? null,
			/** @param {string} key @param {string} value */
			setItem: (key, value) => void items.set(String(key), String(value)),
			/** @param {string} key */
			removeItem: (key) => void items.delete(String(key)),
			clear: () => items.clear(),
			/** @param {number} index */
			key: (index) => [...items.keys()][index] ?? null,
			get length() {
				return items.size;
			}
		},
		{
			get: (target, key) => (key in target || typeof key !== 'string' ? /** @type {any} */ (target)[key] : items.get(key)),
			set: (target, key, value) => (typeof key === 'string' && !(key in target) ? (items.set(key, String(value)), true) : Reflect.set(target, key, value)),
			deleteProperty: (_, key) => (typeof key === 'string' ? items.delete(key) : false) || true,
			has: (target, key) => key in target || (typeof key === 'string' && items.has(key))
		}
	);
}

let page_title = '';

const document = {
	nodeType: 9,
	get baseURI() {
		return address.href;
	},
	get URL() {
		return address.href;
	},
	location,
	// The page's title is the window's. Changing the window needs
	// `permissions.window`; without it the title is only remembered here.
	get title() {
		return page_title;
	},
	set title(value) {
		page_title = String(value);
		host_call('window.set_title', { title: page_title }).catch(() => {});
	},
	nodeName: '#document',
	/** @type {Map<string, Function[]> | null} */
	listeners: null,
	body: root,
	documentElement: Object.assign(detached(), { name: 'html' }),
	head: Object.assign(detached(), { name: 'head' }),
	defaultView: g,
	visibilityState: 'visible',
	hidden: false,
	readyState: 'complete',
	dir: 'ltr',
	get activeElement() {
		// What had the focus may have left the page since.
		if (active !== null && !(/** @type {any} */ (root).contains(active))) active = null;
		return active ?? root;
	},
	hasFocus: () => true,
	/** @param {string} name */
	createElement: (name) => renderer.createElement(name),
	/** @param {string} _namespace @param {string} name */
	createElementNS: (_namespace, name) => renderer.createElement(name),
	/** @param {string} data */
	createTextNode: (data) => renderer.createText(data),
	/** @param {string} data */
	createComment: (data) => renderer.createComment(data),
	createDocumentFragment: () => renderer.createFragment(),
	/** @param {string} name */
	getElementsByTagName: (name) => query_all(root, name === '*' ? '*' : name.toLowerCase()),
	/** @param {string} id */
	getElementById: (id) => {
		let found = null;
		walk(root, (node) => {
			if (node.attrs?.get('id') !== id) return false;
			found = node;
			return true;
		});
		return found;
	},
	/** @param {string} selector */
	querySelector: (selector) => (matches(root, selector) ? root : query(root, selector)),
	/** @param {string} selector */
	querySelectorAll: (selector) => query_all(root, selector),
	/** @param {any} node */
	contains: (node) => /** @type {any} */ (root).contains(node),
	getRootNode: () => document,
	/** @param {string} type @param {any} handler @param {any} [options] */
	addEventListener(type, handler, options) {
		listen(document, type, handler, options);
	},
	/** @param {string} type @param {any} handler @param {any} [options] */
	removeEventListener(type, handler, options) {
		unlisten(document, type, handler, options);
	},
	/** @param {any} event */
	dispatchEvent(event) {
		event.target = document;
		for (const handler of (document.listeners?.get(event.type) ?? []).slice()) handler.call(document, event);
		return !event.defaultPrevented;
	}
};
document.documentElement.parent = null;
host_hooks.outside = document.documentElement;
// <html> is not in the tree (the host's root is <body>), but it is asked
// whether a node is in the page.
/** @type {any} */ (document.documentElement).contains = (/** @type {any} */ node) => node === document.documentElement || /** @type {any} */ (root).contains(node);
// <html> is the viewport: the same numbers as `innerWidth` and `innerHeight`,
// so that their difference, the width of a scrollbar, comes out as zero.
define(document.documentElement, { clientWidth: { get: () => viewport.width }, clientHeight: { get: () => viewport.height } });
// Measuring <body> from the start is what makes the host report the viewport.
reported_box(root);

/** `window` is the global object; these are its listeners. */
const window_target = { listeners: /** @type {Map<string, Function[]> | null} */ (null) };
// Above <body>: <html>, where routers listen for clicks on links, then the
// document, then the window.
outer_targets.push(document.documentElement, document, window_target);

// The host moves focus on its own when the user clicks or tabs. Follow it.
add_listener(document, 'focusin', (/** @type {any} */ event) => {
	active = event.target;
});
add_listener(document, 'focusout', (/** @type {any} */ event) => {
	if (active === event.target) active = null;
});

/** Never calls back: nothing here watches the tree, or what is on screen. */
class Observer {
	observe() {}
	unobserve() {}
	disconnect() {}
	takeRecords() {
		return [];
	}
}

/** Blank answers for every property: styles are computed in the host. */
const computed_style = new Proxy(
	{},
	{
		get: (_, name) => (name === 'getPropertyValue' ? () => '' : name === 'length' ? 0 : typeof name === 'string' ? '' : undefined)
	}
);

/** Install a global unless the engine or the app already has it. @param {Record<string, any>} values */
function provide(values) {
	for (const name of Object.keys(values)) if (g[name] === undefined) g[name] = values[name];
}

provide({
	window: g,
	self: g,
	document,
	navigator: { userAgent: 'sepal', platform: '', language: 'en-US', languages: ['en-US'], maxTouchPoints: 0, onLine: true },
	location,
	history,
	sessionStorage: memory_storage(),
	URL,
	URLSearchParams,
	TextEncoder,
	TextDecoder,
	PopStateEvent,
	HashChangeEvent: Event,
	devicePixelRatio: 1,
	scrollX: 0,
	scrollY: 0,
	pageXOffset: 0,
	pageYOffset: 0,
	/** @param {string} type @param {any} handler @param {any} [options] */
	addEventListener: (type, handler, options) => listen(window_target, type, handler, options),
	/** @param {string} type @param {any} handler @param {any} [options] */
	removeEventListener: (type, handler, options) => unlisten(window_target, type, handler, options),
	/** @param {any} event */
	dispatchEvent: (event) => {
		event.target = g;
		for (const handler of (window_target.listeners?.get(event.type) ?? []).slice()) handler.call(g, event);
		return !event.defaultPrevented;
	},
	getComputedStyle: () => computed_style,
	/** @param {string} query */
	matchMedia: (query) => ({ matches: false, media: query, addEventListener() {}, removeEventListener() {}, addListener() {}, removeListener() {} }),
	getSelection: () => null,
	scrollTo() {},
	Event,
	CustomEvent,
	UIEvent: Event,
	MouseEvent,
	PointerEvent,
	KeyboardEvent,
	FocusEvent,
	InputEvent,
	EventTarget,
	AbortController,
	AbortSignal,
	Node: Object.assign(
		node_class(() => true),
		{ ELEMENT_NODE: 1, TEXT_NODE: 3, COMMENT_NODE: 8, DOCUMENT_NODE: 9, DOCUMENT_FRAGMENT_NODE: 11, DOCUMENT_POSITION_PRECEDING: 2, DOCUMENT_POSITION_FOLLOWING: 4, DOCUMENT_POSITION_CONTAINS: 8, DOCUMENT_POSITION_CONTAINED_BY: 16 }
	),
	Element: node_class(is_element),
	HTMLElement: node_class((node) => is_element(node) && !SVG_TAGS.test(node.name)),
	SVGElement: node_class((node) => is_element(node) && SVG_TAGS.test(node.name)),
	HTMLInputElement: tag_class('input'),
	HTMLTextAreaElement: tag_class('textarea'),
	HTMLSelectElement: tag_class('select'),
	HTMLButtonElement: tag_class('button'),
	HTMLAnchorElement: tag_class('a'),
	// An <a> inside an <svg>. Elements here do not know their namespace, and
	// such a link is given the HTML one (see `SVG_ELEMENTS` in the host).
	SVGAElement: node_class(() => false),
	HTMLFormElement: tag_class('form'),
	HTMLLabelElement: tag_class('label'),
	Document: class {
		static [Symbol.hasInstance](/** @type {any} */ value) {
			return value === document;
		}
	},
	ShadowRoot: class {},
	DocumentFragment: node_class((node) => node.type === 'fragment'),
	Text: node_class((node) => node.type === 'text'),
	ResizeObserver,
	MutationObserver: Observer,
	IntersectionObserver: Observer,
	DOMRect,
	DOMRectReadOnly: DOMRect,
	CSS: { escape: (/** @type {string} */ text) => String(text).replace(/[^\w-]/g, (c) => '\\' + c), supports: () => false }
});
if (g.innerWidth === undefined) {
	define(g, { innerWidth: { get: () => viewport.width }, innerHeight: { get: () => viewport.height } });
}
