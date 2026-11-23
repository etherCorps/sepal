// Headless check of the scenarios' flows (compare/shared/PROTOCOL.md) through GPUI's real hit
// testing, on the seed (5,000 issues): `npm test` (gpuix-svelte --bun test.ts). macOS caps the
// headless viewport at ~540 px high, so this is not the 1200x800 window.
import { mount_headless, click_test_id, find_test_id, press, type, wait, wait_for, all_text, wheel, center, check, finish, text_content } from 'gpuix-svelte/test';
import { router } from 'gpuix-svelte-ui';
import App from './src/App.svelte';
import { data, get_issue } from './src/store.svelte.ts';

mount_headless(App, { width: 1200, height: 800 });
await wait(50);

const title = () => text_content(find_test_id('page-title')!);
check('opens on Issues', title(), 'Issues');
check('rows are windowed, not all 200 paged', all_text().filter((t) => /^[A-Z]{3}-\d+$/.test(t)).length < 60, true);

// 5: click a row, then Escape
const first = data.paged.rows[0].key;
click_test_id('row');
await wait(20);
check('a row click opens its pane', router.path, `/issues/${first}`);
check('the pane is there', find_test_id('pane') !== null, true);
press('escape');
await wait(20);
check('Escape closes the pane', router.path, '/issues');

// j moves the highlight
press('j');
await wait(20);
check('j highlights a row', find_test_id('row-active') !== null, true);

// 6: search
click_test_id('search');
type('d e p l o y');
await wait(50);
check('the search filters the list', data.filtered.length > 0 && data.filtered.every((i) => `${i.key} ${i.title}`.toLowerCase().includes('deploy')), true);
type('space c a c');
await wait(50);
// The seed has no "deploy cac…" title: the list shows its empty state, as the Tracker's does.
check('"deploy cac" empties the list', find_test_id('empty') !== null, true);
press('escape');
for (let i = 0; i < 10; i++) press('backspace');

// 7: status of an open issue, through the Select
click_test_id('search');
for (let i = 0; i < 12; i++) press('backspace');
await wait(20);
const target = data.paged.rows.find((i) => i.status !== 'done')!.key;
const { navigate } = await import('gpuix-svelte-ui');
await navigate(`/issues/${target}`);
await wait(20);
click_test_id('status-select-trigger');
await wait(20);
click_test_id('status-select-done');
await wait(20);
check('the Select item sets the status', get_issue(target)?.status, 'done');

// 8: New issue, Cmd+Enter
const before = data.total;
click_test_id('nav-new');
await wait(20);
type('W r i t e space t h e space d o c s');
press('cmd-enter');
await wait(50);
check('Cmd+Enter creates the issue', data.total, before + 1);
check('a toast says so', all_text().some((t) => t.startsWith('Created ')), true);

// 9: Show all, then wheel: the window follows the scroll
await navigate('/issues');
await wait(20);
// Show all sits under 200 rows, below the headless viewport: the store call its button makes.
(await import('./src/store.svelte.ts')).show_all();
await wait(50);
const list = find_test_id('list')!;
const at = center(list);
const top_before = data.paged.rows.findIndex((i) => all_text().includes(i.key));
for (let i = 0; i < 20; i++) wheel(at, 0, -400);
await wait(50);
const keys = new Set(all_text());
const top_after = data.paged.rows.findIndex((i) => keys.has(i.key));
check('wheeling down mounts rows further down', top_after > top_before + 100, true);

// 10: screens
for (const [id, name] of [['nav-board', 'Board'], ['nav-insights', 'Insights'], ['nav-settings', 'Settings'], ['nav-issues', 'Issues']]) {
	click_test_id(id);
	await wait_for(() => title() === name, { label: name });
	check(`${id} shows ${name}`, title(), name);
}

finish('tracker-gpuix', 16);
