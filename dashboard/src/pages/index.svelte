<script>
    import { goto } from "@roxi/routify";
    import { metatags } from "@roxi/routify";
    import ArrowRight32 from "carbon-icons-svelte/lib/ArrowRight32";
    metatags.title = "WebGrid";
    metatags.description =
        "Decentralized, scalable and robust implementation of a selenium-grid equivalent. Based on the WebDriver specification by the W3C.";

    let sessionID;
</script>

<div class="w-screen h-screen flex flex-col justify-center items-center">
    <img
        class="w-16 h-16 lg:w-32 lg:h-32 rounded-full"
        src="https://placekitten.com/300/300"
        alt="WebGrid Logo"
    />
    <h2 class="lg:text-5xl text-3xl p-4 lg:p-8">WebGrid</h2>
    <form
        on:submit={() => {
            if (sessionID) $goto(`./watch/${sessionID}`);
        }}
    >
        <div class="flex flex-col justify-center items-center">
            <input
                type="text"
                placeholder="Enter your Session ID"
                class="p-4 bg-gray-100 rounded-lg"
                style="width: 37ch"
                bind:value={sessionID}
            />
            <button type="submit" class="p-4">
                <ArrowRight32 />
            </button>
        </div>
    </form>
</div>

<div class="absolute bottom-0 pb-4 text-center w-full text-xs">
    For now you can just view sessions by their ID.
    <br />
    Future functionality coming soon.
</div>
