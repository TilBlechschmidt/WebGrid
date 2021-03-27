<!-- routify:options preload="proximity" -->
<script>
    import Styles from "../components/Styles.svelte";
    import { WebGridAPI } from "../data/api";

    // Initialize the API (stores the relevant bits in the Svelte context)
    new WebGridAPI();
    console.log("revision", __buildRevision__);
</script>

<Styles />

<div class="webgrid">
    <slot />
</div>

<style global lang="postcss">
    @tailwind base;
</style>
