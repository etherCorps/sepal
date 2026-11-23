import { defineConfig } from '@ethercorps/svelte-blitz-vite';

// The file router and the DOM layer together, under Bits UI.
// crates/host/tests/router_dom.rs drives it.
export default defineConfig({
	app: { name: 'router + dom probe', id: 'com.example.router-dom-probe', window: { width: 900, height: 700 } },
	dom: true
});
