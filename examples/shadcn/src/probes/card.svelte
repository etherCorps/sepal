<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '$lib/components/ui/card/index.js';
</script>

<div class="p-6">
	<Card class="w-80">
		<CardHeader>
			<CardTitle>Create project</CardTitle>
			<CardDescription>Deploy your new project in one click.</CardDescription>
		</CardHeader>
		<CardContent><p>Card content goes here.</p></CardContent>
		<CardFooter><Button>Deploy</Button></CardFooter>
	</Card>
</div>
