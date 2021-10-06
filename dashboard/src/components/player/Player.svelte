<script lang="ts">
    import { onMount } from "svelte";
    import Hls from "../../hls.esm";
    import StandbyScreen from "./StandbyScreen.svelte";
    import Controls from "./Controls.svelte";

    export let url: string;
    export let subtitleURL: string = url.replace(".m3u8", ".vtt");
    export let sessionID: string;

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

    function handleKeypress(e) {
        if (e.code == "Space") {
            paused = !paused;
            e.preventDefault();
            return false;
        } else if (!live && e.code == "ArrowLeft") {
            paused = true;
            currentTime -= e.shiftKey ? 10 : 1 / 15;
            e.preventDefault();
            return false;
        } else if (!live && e.code == "ArrowRight") {
            paused = true;
            currentTime += e.shiftKey ? 10 : 1 / 15;
            e.preventDefault();
            return false;
        }
    }

    onMount(async () => {
        const hls = new Hls({});

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

        document.addEventListener("keydown", handleKeypress);
        return () => {
            document.removeEventListener("keydown", handleKeypress);
        };
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
    crossorigin="anonymous"
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
    on:click={() => (paused = !paused)}
>
    {#if showPlayerUI}
        <div class="absolute bottom-0 w-full" on:click|stopPropagation>
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
