import { defineConfig } from 'sepal/vite';

// SvelteKit's router and the DOM layer together, under Bits UI.
// crates/host/tests/router_dom.rs drives it.
export default defineConfig({
	app: { name: 'router + dom probe', id: 'com.example.router-dom-probe', window: { width: 900, height: 700 } }
});
