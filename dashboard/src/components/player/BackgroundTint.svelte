<script lang="ts">
    import { fade } from "svelte/transition";
</script>

<div class="background-tint" out:fade>
    <div class="blur" />
    <div class="tint" />
</div>

<style>
    .background-tint,
    .background-tint > div {
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        border-radius: 8px;
        pointer-events: none;
    }

    .background-tint > .blur {
        background-color: rgba(0, 0, 0, 0.55);
        backdrop-filter: saturate(180%) blur(17.5px);
    }

    .background-tint > .tint {
        background-color: rgba(255, 255, 255, 0.14);
        mix-blend-mode: lighten;
    }
</style>
