<script lang="ts">
    import { onMount } from "svelte";
    import { taurpc } from "./lib/rpc";

    onMount(() => {
        test();
    });
    const test = async () => {
        const res = await taurpc.test_event("input1", 2);
        console.log(res);
    };
</script>

<main class="container">
    <h1>Test</h1>
</main>
