<script lang="ts">
    import { getContext, onMount } from "svelte";
    import { WebGridAPI } from "../data/api";

    export let loadEmbedStyles: boolean = false;

    if (loadEmbedStyles) {
        // Build the href and make sure we haven't injected the styles previously
        const href = `${getContext(WebGridAPI.HostKey)}/embed.css`;
        const injected = Array.from(
            document.getElementsByTagName("link")
        ).reduce((acc, link) => acc || link.href == href, false);

        // Inject the styles if required
        if (!injected) {
            console.log("WebGrid Embed: Injecting required stylesheets.");

            var link = document.createElement("link");
            link.href = href;
            link.type = "text/css";
            link.rel = "stylesheet";
            link.id = "_webgrid_injected_styles";

            document.getElementsByTagName("head")[0].appendChild(link);
        }
    }
</script>

<style global lang="postcss">
    .webgrid {
        /* Manually include style resets that would otherwise apply to <html> and <body> */
        line-height: 1.15;
        -webkit-text-size-adjust: 100%;
        font-family: system-ui, -apple-system, "Segoe UI", Roboto, Helvetica,
            Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji";

        @tailwind base;
    }
    @tailwind components;
    @tailwind utilities;
</style>
