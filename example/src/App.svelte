<script lang="ts">
    import { taurpc } from "./lib/rpc";
    import { onMount, onDestroy } from "svelte";
    import { defineResolvers } from "../../src";

    let value = "";
    const call_backend = async () => {
        await taurpc.update_state(value);
        await taurpc.get_window();
        // console.log("before sleep");
        // await taurpc.with_sleep();
        // console.log("after sleep");

        // await taurpc.test_option();
        try {
            const res = await taurpc.test_result({
                first_name: "",
                last_name: "",
                uid: 1,
            });
            console.log(res);
        } catch (error) {
            console.error(error);
            // Handle error
        }
    };

    let cleanup = null;

    onMount(async () => {
        const { unsubscribe, on } = await defineResolvers();
        cleanup = unsubscribe;

        on("update_state", (value) => {
            console.log(value);
        });
    });

    onDestroy(() => {
        cleanup();
    });
</script>

<main class="container">
    Set managed state on backend <input type="text" bind:value />
    <button on:click={call_backend}>Call Backend code</button>
</main>
