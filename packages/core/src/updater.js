// Self-update for bundled apps.
//
//   import { check } from '@ethercorps/svelte-blitz/updater';
//
//   const update = await check();
//   if (update) {
//     await update.download();   // fetched and its signature verified
//     await update.install();    // replaces the app and starts the new version
//   }
//
// The feed URL and the public key updates must be signed with come from the
// `updater` block of the app manifest (see the Vite preset's `app` option).

import { host_call } from './host-call.js';

/**
 * A version newer than the running one.
 *
 * @typedef {{
 *   version: string,
 *   current_version: string,
 *   notes: string | null,
 *   date: string | null,
 *   download: () => Promise<void>,
 *   install: () => Promise<never>
 * }} Update
 *
 * `download()` fetches the update and verifies its signature; nothing on disk
 * changes. `install()` downloads if that has not happened yet, replaces the
 * app and relaunches it, so on success its promise never settles. It rejects
 * when the app is not running from a bundle (in dev, for instance).
 */

/**
 * Ask the update feed whether there is a newer version for this platform.
 * Rejects if the feed cannot be reached or the app has no `updater` configured.
 *
 * @returns {Promise<Update | null>} `null` when the app is up to date
 */
export async function check() {
	const found = await host_call('updater.check');
	if (found === null) return null;
	return {
		...found,
		download: () => host_call('updater.download'),
		install: () => host_call('updater.install')
	};
}
