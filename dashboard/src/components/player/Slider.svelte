<script lang="ts">
    import { createEventDispatcher } from "svelte";

    export let position = 0;
    export let buffered = 0;
    export let max = 1;
    export let disabled = false;

    const dispatch = createEventDispatcher();

    $: progress = position / max;
    $: primaryStyle = `width: ${progress * 100}%`;
    $: secondaryStyle = `width: ${buffered * 100 || 100}%`;
    $: knobStyle = `left: ${progress * 100}%`;
</script>

<div
    class="slider"
    on:mousedown={() => dispatch("dragstart")}
    on:mouseup={() => dispatch("dragend")}
>
    <div class="custom-slider">
        <div class="track fill" />
        <div class="secondary fill" style={secondaryStyle} />
        <div class="primary fill" style={primaryStyle} />
        {#if !disabled}
            <div class="knob" style={knobStyle} />
        {/if}
    </div>
    <input
        type="range"
        min="0"
        {max}
        step="0.001"
        bind:value={position}
        {disabled}
    />
</div>

<style lang="postcss">
    .slider {
        margin-top: 1.5px;
        position: relative;
        flex-grow: 1;
    }

    .slider > input[type="range"] {
        width: 100%;
        appearance: none;
        background-color: transparent;
        outline: none;
        border: none;
    }

    .slider > .custom-slider {
        pointer-events: none;
        position: absolute;
        left: 0;
        width: 100%;
        height: 5px;
        top: 5.5px;
    }

    .custom-slider > * {
        position: absolute;
        pointer-events: none;
    }

    .track {
        height: 100%;
        width: 100%;
        background-color: rgba(255, 255, 255, 0.35);
    }

    .primary {
        background-color: rgba(255, 255, 255, 0.2);
    }

    .secondary {
        background-color: rgba(255, 255, 255, 0.2);
    }

    .fill {
        height: 100%;
        border-radius: 4.5px;
        mix-blend-mode: plus-lighter;
    }

    .knob {
        top: -2px;
        width: 9px;
        height: 9px;
        border-radius: 50%;
        background-color: white;
        transform: translateX(-50%);
    }

    .slider > input {
        top: 0;
        margin: 0;
        height: 100%;
        background-color: transparent;
        -webkit-appearance: none !important;

        outline: none;
    }

    .slider > input::-webkit-slider-thumb {
        width: 9px;
        height: 100%;
        border: none;
        box-shadow: none;
        background-color: transparent;
        -webkit-appearance: none !important;
        pointer-events: all;
    }

    .slider > input::-moz-range-thumb {
        width: 9px;
        height: 100%;
        border: none;
        box-shadow: none;
        background-color: transparent;
        -webkit-appearance: none !important;
        pointer-events: all;
    }

    .slider > input::-ms-thumb {
        width: 9px;
        height: 100%;
        border: none;
        box-shadow: none;
        background-color: transparent;
        -webkit-appearance: none !important;
        pointer-events: all;
    }
</style>
