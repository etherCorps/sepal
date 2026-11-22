import { defineConfig, loadEnv } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { svelteBlitz } from '@ethercorps/svelte-blitz-vite';

// The version, feed and key come from the environment so that one source tree
// can be built as "the installed version" and as "the update" (see
// scripts/updater-e2e.sh). A real app writes them here.
export default defineConfig(({ mode }) => {
	const env = loadEnv(mode, '.', '');
	const version = env.APP_VERSION ?? '0.1.0';

	return {
		define: { __APP_VERSION__: JSON.stringify(version) },
		plugins: [
			tailwindcss(),
			svelteBlitz({
				app: {
					name: 'Updater Demo',
					id: 'com.example.updater-demo',
					version,
					icon: 'icon.png',
					window: { width: 520, height: 360 },
					updater: {
						// Where latest.json is published.
						url: env.UPDATE_URL ?? 'https://example.com/updates/latest.json',
						// From `svelte-blitz signer generate`. This one is a placeholder.
						pubkey: env.UPDATE_PUBKEY ?? 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA='
					}
				}
			})
		]
	};
});
