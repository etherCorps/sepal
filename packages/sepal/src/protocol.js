// Wire protocol between the JS renderer and the Rust host.
//
// A batch is { ops: Int32Array, strings: string[] }. `ops` is a flat list of
// records: an opcode followed by a fixed number of integer operands. Operands
// are node ids or indexes into `strings`. Node ids are never reused: the host
// numbers the nodes it parsed from index.html (1 to n, handed to JS once as
// `__sb_page`), and JS allocates the rest, from n + 1.
//
// Keep this file in sync with crates/host/src/ops.rs.

export const OP = {
	CREATE_ELEMENT: 1, // id, name
	CREATE_TEXT: 2, // id, text
	CREATE_COMMENT: 3, // id, text
	SET_TEXT: 4, // id, text
	SET_ATTR: 5, // id, name, value
	REMOVE_ATTR: 6, // id, name
	INSERT: 7, // parent, node, anchor (0 = append)
	REMOVE: 8, // node (detach, keep alive)
	DROP: 9, // node (JS object was garbage collected; free it)
	LISTEN: 10, // type (first listener for this event type anywhere)
	UNLISTEN: 11, // type (last listener for this event type removed)
	MEASURE: 12, // node (report its box after every layout that changes it)
	FOCUS: 13, // node (move the keyboard focus to it; 0 = to nothing)
	SCROLL: 14, // node, left, top (scroll it to that offset; the host clamps it)
	SCROLL_INTO_VIEW: 15 // node, block, inline (scroll its ancestors; 0 start, 1 center, 2 end, 3 nearest)
};

// Flags returned from dispatch
export const DISPATCH_PREVENT_DEFAULT = 1;
export const DISPATCH_STOP_PROPAGATION = 2;
