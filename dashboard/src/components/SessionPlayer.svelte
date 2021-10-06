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
        query($sessionID: Uuid!) {
            session {
                fetch(id: $sessionID) {
                    video {
                        playlist
                    }
                }
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
            !($session.data.session && $session.data.session.fetch))
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
        : !($session.data.session && $session.data.session.fetch);
</script>

<div class="relative w-full h-full player-frame bg-black">
    {#if loading || error}
        <div>
            <StandbyScreen message={sessionID} {loading} {error} />
        </div>
    {:else}
        <Player
            {sessionID}
            url={host + $session.data.session.fetch.video.playlist}
        />
    {/if}
</div>
