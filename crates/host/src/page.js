// Run by the host before any script of the page, with the page as it was
// parsed (page.rs). renderer.js builds its mirror of the tree from
// `__sb_page` once, when it loads.
(page) => {
	const g = globalThis;
	g.__sb_page = page;

	// The host's way in, until the app's runtime (the `sepal` package)
	// replaces it: a page that has not loaded it yet has nothing to render,
	// and what its promises reject with is still reported.
	const rejected = new Map();
	g.__sb = {
		take: () => null,
		dispatch: () => 0,
		timers: () => false,
		next_timer: () => -1,
		frame: () => false,
		frame_pending: () => false,
		resolve() {},
		layout() {},
		rejection(promise, reason, handled) {
			if (handled) rejected.delete(promise);
			else rejected.set(promise, reason);
		},
		rejections() {
			for (const reason of rejected.values()) g.__sb_host.log(2, `Uncaught (in promise) ${reason?.stack ?? reason}`);
			rejected.clear();
		}
	};

	// `document.currentScript` for a classic script that runs before the DOM
	// layer has loaded. That is how a SvelteKit page starts: its script takes
	// the element it sits in, then imports the app. The objects handed out
	// here become the mirror's nodes for those ids when renderer.js loads, so
	// the element the script took is the one the app mounts into.
	const held = (page.held = new Map());
	const placeholder = {
		get parentElement() {
			return node(page.parents[this.node_id]);
		}
	};
	const node = (id) => {
		if (!id) return null;
		let found = held.get(id);
		if (found === undefined) held.set(id, (found = Object.assign(Object.create(placeholder), { node_id: id })));
		return found;
	};
	page.script = 0;
	// Around each classic script. The stand-in `document` exists only while
	// one runs: an app without the DOM layer must not find one.
	page.begin = (id) => {
		page.script = id;
		if (g.document === undefined) {
			g.document = {
				sepal_placeholder: true,
				get currentScript() {
					return node(page.script);
				}
			};
		}
	};
	page.end = () => {
		page.script = 0;
		if (g.document?.sepal_placeholder) delete g.document;
	};
}
