<script lang="ts">
    import { onMount } from "svelte";
    import { taurpc } from "./lib/rpc";

    onMount(() => {
        call_backend();
    });

    const call_backend = async () => {
        const res = await taurpc.test_event("hello world", 0);
        console.log(res);
    };
</script>

<main class="container">
    <h1>Test</h1>
    <button on:click={call_backend}>Call Backend code</button>
</main>
