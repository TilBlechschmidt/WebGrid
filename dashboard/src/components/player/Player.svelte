<script lang="ts">
    import { onMount } from "svelte";
    import Hls from "hls.js";
    import StandbyScreen from "./StandbyScreen.svelte";
    import Controls from "./Controls.svelte";

    export let url: string;
    export let subtitleURL: string = url.replace(".m3u8", ".vtt");
    export let sessionID: string;

    const hls = new Hls();
    let video;
    let error;

    let showPlayerUI = false;

    let loaded = false;
    let live = true;
    let paused = true;
    let currentTime;
    let duration;
    let ended;
    let buffered;

    onMount(async () => {
        hls.on(
            Hls.Events.LEVEL_LOADED,
            (_, data) => (live = data.details.live)
        );

        hls.on(Hls.Events.ERROR, (_, data) => {
            console.error("Encountered error while loading video", data);
            if (data.fatal) {
                error = data;
            }
        });

        video.addEventListener("loadeddata", () => {
            error = undefined;

            if (!loaded) {
                showPlayerUI = true;
                loaded = true;
                video.play();
            }
        });

        hls.attachMedia(video);
        hls.loadSource(url);
    });

    $: showStandbyScreen = ended || !loaded || error;
    $: standbyMessage = ended ? "Session has terminated." : undefined;
    $: controlsDisabled = !loaded;
</script>

<!-- svelte-ignore a11y-media-has-caption -->
<video
    bind:this={video}
    bind:paused
    bind:duration
    bind:currentTime
    bind:ended
    bind:buffered
    class="absolute inset-0 w-full max-h-screen"
>
    <track
        kind="subtitles"
        label="WebGrid Messages"
        srclang="en"
        src={subtitleURL}
        default
    />
</video>

{#if showStandbyScreen}
    <div class="absolute inset-0 w-full h-full max-h-screen">
        <StandbyScreen
            message={standbyMessage || sessionID}
            loading={standbyMessage == undefined && !error}
            {error}
        />
    </div>
{/if}

<div
    class="absolute inset-0 w-full h-full max-h-screen"
    on:mouseenter={() => (showPlayerUI = true)}
    on:mouseleave={() => (showPlayerUI = paused)}
>
    {#if showPlayerUI}
        <div class="absolute bottom-0 w-full">
            <Controls
                bind:position={currentTime}
                bind:duration
                bind:paused
                {live}
                disabled={controlsDisabled}
                buffered={buffered.end}
                tintBackground={!showStandbyScreen}
            />
        </div>
    {/if}
</div>
