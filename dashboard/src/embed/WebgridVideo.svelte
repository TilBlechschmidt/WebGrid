<script lang="ts">
    import SessionPlayer from "../components/SessionPlayer.svelte";
    import Styles from "../components/Styles.svelte";
    import { WebGridAPI } from "../data/api";

    export let sessionID: string;
    export let host: string | undefined;

    new WebGridAPI(host);
</script>

<Styles loadEmbedStyles />

<div class="webgrid">
    <SessionPlayer {sessionID} />
</div>
