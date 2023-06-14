<script lang="ts">
    import { taurpc } from "./lib/rpc.ts";

    const call_backend = async () => {
        const res = await taurpc.test_event("hello world2", 0);
        
        console.log(res);
    };
</script>

<main class="container">
    <h1>Test</h1>
    <button on:click={call_backend}>Call Backend code</button>
</main>
