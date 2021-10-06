<script lang="ts">
    import ClosedCaptionAlt16 from "carbon-icons-svelte/lib/ClosedCaptionAlt16";
    import PlayFilledAlt16 from "carbon-icons-svelte/lib/PlayFilledAlt16";
    import PauseFilled16 from "carbon-icons-svelte/lib/PauseFilled16";
    import BackgroundTint from "./BackgroundTint.svelte";
    import Slider from "./Slider.svelte";

    export let live;
    export let paused;
    export let position;
    export let buffered;
    export let duration;
    export let tintBackground = true;
    export let disabled = false;

    function formatSeconds(seconds) {
        if (seconds == undefined || isNaN(seconds) || !isFinite(seconds))
            return "-:--";

        let start = 14;
        let length = 5;

        if (duration > 3600) {
            start = 11;
            length = 8;
        }

        return new Date(1000 * seconds).toISOString().substr(start, length);
    }

    let playingBeforeDrag;

    function onDragStart() {
        playingBeforeDrag = !paused;
        paused = true;
    }

    function onDragEnd() {
        if (playingBeforeDrag) paused = false;
    }

    $: elapsed = formatSeconds(position);
    $: remaining = `-${formatSeconds(Math.max(0, duration - position))}`;
</script>

<div class="rounded-lg relative controls-bar">
    {#if tintBackground}
        <BackgroundTint />
    {/if}
    <div class="relative w-full flex space-x-3 pl-3 pr-3">
        <button {disabled} on:click={() => (paused = !paused)}>
            {#if paused}
                <PlayFilledAlt16 />
            {:else}
                <PauseFilled16 />
            {/if}
        </button>
        {#if live}
            <div class="flex-grow text">Live Broadcast</div>
        {:else}
            <div class="secondary-glyph time text">{elapsed}</div>
            <Slider
                bind:position
                bind:buffered
                max={duration}
                {disabled}
                on:dragstart={onDragStart}
                on:dragend={onDragEnd}
            />
            <div class="secondary-glyph time text">{remaining}</div>
        {/if}
        <button {disabled}>
            <ClosedCaptionAlt16 />
        </button>
    </div>
</div>

<style lang="postcss">
    button {
        margin: 0;
        padding: 0;
        border: none;
    }

    button:active {
        background-color: initial;
    }

    button:disabled {
        color: rgba(255, 255, 255, 0.25);
    }

    .controls-bar {
        font-size: 12px;
        margin: 6px;
        padding: 6px;
        height: 31px;
        --primary-glyph-color: rgba(255, 255, 255, 0.75);
        --secondary-glyph-color: rgba(255, 255, 255, 0.55);
        color: var(--primary-glyph-color);
    }

    .secondary-glyph {
        color: var(--secondary-glyph-color);
        mix-blend-mode: plus-lighter;
    }

    .text {
        line-height: 19px;
        pointer-events: none;
        user-select: none;
    }

    .time {
        font-family: -apple-system-monospaced-numbers;
        font-feature-settings: "tnum";
    }
</style>
