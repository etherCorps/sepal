<!--
	The shadcn-svelte components that work in svelte-blitz today. The interactive
	ones (Switch, Checkbox, Tabs, Accordion, Dialog) need `dom: true` in app.config.ts.
	`node probe.mjs` tries every file in src/probes, including the components
	that do not work yet.
-->
<script lang="ts">
	import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from '$lib/components/ui/accordion/index.js';
	import { Alert, AlertDescription, AlertTitle } from '$lib/components/ui/alert/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button, buttonVariants } from '$lib/components/ui/button/index.js';
	import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '$lib/components/ui/card/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from '$lib/components/ui/dialog/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Progress } from '$lib/components/ui/progress/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';
	import { Tabs, TabsContent, TabsList, TabsTrigger } from '$lib/components/ui/tabs/index.js';

	let clicks = $state(0);
	let name = $state('Ada');
	let on = $state(false);
	let agreed = $state(false);
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
		<Dialog>
			<DialogTrigger id="open-dialog" class={buttonVariants({ variant: 'outline' })}>Open dialog</DialogTrigger>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>Are you sure?</DialogTitle>
					<DialogDescription>Click outside this box, or its cross, to close it.</DialogDescription>
				</DialogHeader>
			</DialogContent>
		</Dialog>
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
				<AlertDescription>Selects and tooltips need to measure the page and do not work yet.</AlertDescription>
			</Alert>
		</div>
	</div>

	<div class="flex gap-6">
		<div class="flex w-80 flex-col gap-3">
			<div class="flex items-center gap-3">
				<Switch id="switch" bind:checked={on} />
				<span id="switch-state">Switch is {on ? 'on' : 'off'}</span>
			</div>
			<div class="flex items-center gap-3">
				<Checkbox id="check" bind:checked={agreed} />
				<span id="check-state">{agreed ? 'Checked' : 'Not checked'}</span>
			</div>
			<!-- Not a flex item: the button sits in a line of text inside a padded box. -->
			<div class="rounded-lg border p-4">
				<Button id="inline" variant="outline" onclick={() => clicks++}>Count from here too</Button>
			</div>
		</div>

		<Tabs value="account" class="w-72">
			<TabsList>
				<TabsTrigger value="account">Account</TabsTrigger>
				<TabsTrigger value="password">Password</TabsTrigger>
			</TabsList>
			<TabsContent value="account"><p class="tab-panel">Account panel</p></TabsContent>
			<TabsContent value="password"><p class="tab-panel">Password panel</p></TabsContent>
		</Tabs>

		<Accordion type="single" class="w-72">
			<AccordionItem value="one">
				<AccordionTrigger>Is it accessible?</AccordionTrigger>
				<AccordionContent><p id="answer">Yes. It follows the WAI-ARIA pattern.</p></AccordionContent>
			</AccordionItem>
			<AccordionItem value="two">
				<AccordionTrigger>Is it styled?</AccordionTrigger>
				<AccordionContent>Yes, by Tailwind.</AccordionContent>
			</AccordionItem>
		</Accordion>
	</div>
</main>
