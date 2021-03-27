<script>
    import { metatags, url } from "@roxi/routify";
    import SessionPlayer from "../../components/SessionPlayer.svelte";
    import ArrowLeft16 from "carbon-icons-svelte/lib/ArrowLeft16";

    export let sessionID;
    metatags.title = `WebGrid | Session ${sessionID.substring(0, 6)}`;
    metatags.description = "Video playback of browser session";
</script>

<p class="p-3">
    <a href={$url("../")} class="text-lg"><ArrowLeft16 class="inline" /> Back</a
    >
</p>

<SessionPlayer {sessionID} />
