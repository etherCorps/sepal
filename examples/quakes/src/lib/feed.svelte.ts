// The feed every page shows: one copy, fetched when the preset changes and
// again once a minute. A minute is how often USGS regenerates the files, and
// one wake-up a minute is all an idle window costs.

import { DEFAULT_PRESET, feed as fetch_feed, type PresetId, type Quake } from './usgs';

const REFRESH_MS = 60_000;

class Feed {
	preset = $state<PresetId>(DEFAULT_PRESET);
	quakes = $state.raw<Quake[]>([]);
	title = $state('');
	/** When the data on screen was fetched; 0 before the first answer. */
	fetched = $state(0);
	/** The clock relative times are read against, moved on with each refresh. */
	now = $state(Date.now());
	loading = $state(false);
	error = $state<string | null>(null);
	/** The quake picked on the map or in the list. */
	selected = $state<string | null>(null);

	#timer: ReturnType<typeof setInterval> | null = null;
	#request = 0;

	async refresh() {
		const request = ++this.#request;
		this.loading = true;
		try {
			const result = await fetch_feed(this.preset);
			// A newer request (another preset picked meanwhile) wins.
			if (request !== this.#request) return;
			this.quakes = result.quakes;
			this.title = result.title;
			this.fetched = Date.now();
			this.error = null;
		} catch (error) {
			if (request === this.#request) this.error = (error as Error).message;
		} finally {
			if (request === this.#request) {
				this.loading = false;
				this.now = Date.now();
			}
		}
	}

	choose(preset: PresetId) {
		if (preset === this.preset) return;
		this.preset = preset;
		this.selected = null;
		this.refresh();
	}

	start() {
		if (this.#timer !== null) return;
		this.refresh();
		this.#timer = setInterval(() => this.refresh(), REFRESH_MS);
	}

	stop() {
		if (this.#timer !== null) clearInterval(this.#timer);
		this.#timer = null;
	}

	find(id: string | null): Quake | undefined {
		return id === null ? undefined : this.quakes.find((quake) => quake.id === id);
	}
}

export const feed = new Feed();
