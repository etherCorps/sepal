<!--
	The shadcn-svelte components that work in svelte-blitz today: the ones that
	are markup and Tailwind classes. `node probe.mjs` tries every file in
	src/probes, including the interactive components that do not work yet.
-->
<script lang="ts">
	import { Alert, AlertDescription, AlertTitle } from '$lib/components/ui/alert/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '$lib/components/ui/card/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Progress } from '$lib/components/ui/progress/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';

	let clicks = $state(0);
	let name = $state('Ada');
	// `bind:this` on a component's root element, which shadcn does everywhere.
	let button = $state<HTMLElement | null>(null);
</script>

<main class="bg-background text-foreground flex h-screen flex-col gap-6 p-8">
	<div class="flex items-center gap-3">
		<h1 class="text-2xl font-semibold">shadcn-svelte on Blitz</h1>
		<Badge>works</Badge>
		<Badge variant="secondary">{clicks} clicks</Badge>
		<Badge variant="outline" id="ref">{button ? 'ref bound' : 'no ref'}</Badge>
	</div>

	<div class="flex flex-wrap gap-2">
		<Button id="count" bind:ref={button} onclick={() => clicks++}>Clicked {clicks}</Button>
		<Button variant="secondary">Secondary</Button>
		<Button variant="outline">Outline</Button>
		<Button variant="destructive">Destructive</Button>
		<Button variant="ghost">Ghost</Button>
		<Button variant="link">Link</Button>
	</div>

	<div class="flex gap-6">
		<Card class="w-80">
			<CardHeader>
				<CardTitle>Profile</CardTitle>
				<CardDescription>What the form on the right says.</CardDescription>
			</CardHeader>
			<CardContent class="flex flex-col gap-3">
				<p id="greeting">Hello, {name}.</p>
				<Progress value={Math.min(100, clicks * 10)} />
			</CardContent>
			<CardFooter><Button variant="outline" onclick={() => (clicks = 0)}>Reset</Button></CardFooter>
		</Card>

		<div class="flex w-72 flex-col gap-3">
			<Label for="name">Name</Label>
			<Input id="name" placeholder="Your name" bind:value={name} />
			<Separator />
			<div class="flex items-center gap-3">
				<Skeleton class="size-10 rounded-full" />
				<div class="flex flex-col gap-2"><Skeleton class="h-3 w-40" /><Skeleton class="h-3 w-28" /></div>
			</div>
			<Alert>
				<AlertTitle>Heads up</AlertTitle>
				<AlertDescription>Dialogs, selects and tooltips need a browser underneath and do not work yet.</AlertDescription>
			</Alert>
		</div>
	</div>
</main>
