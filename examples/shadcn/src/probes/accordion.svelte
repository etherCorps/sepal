<script lang="ts">
	import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from '$lib/components/ui/accordion/index.js';
</script>

<div class="w-96 p-6">
	<Accordion type="single">
		<AccordionItem value="one">
			<AccordionTrigger>Is it accessible?</AccordionTrigger>
			<AccordionContent><p id="answer">Yes. It follows the WAI-ARIA pattern.</p></AccordionContent>
		</AccordionItem>
		<AccordionItem value="two">
			<AccordionTrigger>Is it styled?</AccordionTrigger>
			<AccordionContent>Yes.</AccordionContent>
		</AccordionItem>
	</Accordion>
</div>
