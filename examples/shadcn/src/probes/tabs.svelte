<script lang="ts">
	import { Tabs, TabsContent, TabsList, TabsTrigger } from '$lib/components/ui/tabs/index.js';
</script>

<div class="w-96 p-6">
	<Tabs value="account">
		<TabsList>
			<TabsTrigger value="account">Account</TabsTrigger>
			<TabsTrigger value="password">Password</TabsTrigger>
		</TabsList>
		<TabsContent value="account"><p id="panel">Account panel</p></TabsContent>
		<TabsContent value="password"><p id="panel">Password panel</p></TabsContent>
	</Tabs>
</div>
