<script>
    import { url } from "@roxi/routify";
</script>

<div class="w-screen h-screen flex flex-col justify-center items-center">
    <div class="text-9xl">404</div>
    <div class="text-xl">
        Page not found. <a href={$url("../")} class="underline">Go back</a>
    </div>
</div>
