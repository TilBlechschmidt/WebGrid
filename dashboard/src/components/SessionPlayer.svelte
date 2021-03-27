<script lang="ts">
    import { operationStore, query } from "@urql/svelte";
    import StandbyScreen from "./player/StandbyScreen.svelte";
    import Player from "./player/Player.svelte";
    import { getContext } from "svelte";
    import { WebGridAPI } from "../data/api";

    export let sessionID: string;

    const host = getContext(WebGridAPI.HostKey);
    const session = operationStore(
        `
        query($sessionID: String!) {
            session(id: $sessionID) {
                videoURL
            }
        }
    `,
        {
            sessionID,
        }
    );

    query(session);

    $: if (
        !$session.fetching &&
        ($session.error ||
            !($session.data.session && $session.data.session.videoURL))
    ) {
        console.error(
            "Error while requesting video URL",
            $session.error || "No video URL available."
        );
    }

    $: loading = $session.fetching;
    $: error = $session.error
        ? $session.error.message
        : $session.fetching
        ? false
        : !($session.data.session && $session.data.session.videoURL);
</script>

<div class="relative w-full pt-16/9 player-frame">
    {#if loading || error}
        <div class="absolute inset-0 w-full h-full max-h-screen">
            <StandbyScreen message={sessionID} {loading} {error} />
        </div>
    {:else}
        <Player {sessionID} url={host + $session.data.session.videoURL} />
    {/if}
</div>

<style>
    .pt-16\/9 {
        padding-top: 56.25%;
    }

    .player-frame > * {
        @apply absolute;
        @apply inset-0;
        @apply w-full;
        @apply h-full;
        @apply max-h-screen;
    }
</style>
