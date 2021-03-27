<script lang="ts">
    import WarningAltFilled32 from "carbon-icons-svelte/lib/WarningAltFilled32";
    import { fade, fly } from "svelte/transition";
    import Loader from "../Loader.svelte";

    export let loading: Boolean = true;
    export let message: string;
    export let error: boolean | string = false;

    const fadeTransition = { duration: 250 };
    const logoFadeTransition = {
        duration: fadeTransition.duration,
        delay: fadeTransition.duration / 2,
    };
    const flyTransition = {
        y: 50,
        duration: fadeTransition.duration,
        delay: fadeTransition.duration,
    };
</script>

<div
    in:fade={fadeTransition}
    class="w-full h-full flex flex-col justify-center items-center bg-gray-700 pointer-events-none select-none"
>
    <img
        in:fade={logoFadeTransition}
        class="w-16 h-16 lg:w-32 lg:h-32 rounded-full"
        src="http://placekitten.com/300/300"
        alt="WebGrid Logo"
    />
    <h2
        class="lg:text-5xl text-3xl text-white p-4 lg:p-8"
        in:fly={flyTransition}
    >
        WebGrid
    </h2>
    <p
        class="text-xs lg:text-base font-mono text-gray-400"
        in:fly={flyTransition}
    >
        {message}
    </p>
    {#if loading}
        <div class="p-4 lg:p-8" in:fly={flyTransition}><Loader /></div>
    {:else if error}
        <div
            class="p-4 lg:p-8 text-yellow-500 flex items-center space-x-3"
            in:fly={flyTransition}
        >
            <div class="hidden lg:block">
                <WarningAltFilled32 />
            </div>
            <div class="text-xs lg:text-base">
                {typeof error === "string"
                    ? error
                    : "The video is unavailable."}
            </div>
        </div>
    {/if}
</div>
