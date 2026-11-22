import { mount, unmount } from '@ethercorps/svelte-blitz';
import './app.css';
import App from './App.svelte';

const app = mount(App);

declare global {
	// Used by the host's tests to check that removed nodes are freed.
	var __test_unmount: () => void;
}
globalThis.__test_unmount = () => unmount(app);
