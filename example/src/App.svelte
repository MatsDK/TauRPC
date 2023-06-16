<script lang="ts">
    import { taurpc } from "./lib/rpc";

    let value = "";
    const call_backend = async () => {
        const res = await taurpc.test_event("hello world2", 0);
        console.log(res);

        await taurpc.test_state(value);
        await taurpc.test_window();
        await taurpc.test_app_handl();
    };
</script>

<main class="container">
    Set managed state on backend <input type="text" bind:value />
    <button on:click={call_backend}>Call Backend code</button>
</main>
