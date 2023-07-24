<script lang="ts">
    import { taurpc } from "./lib/ipc";
    import { onMount, onDestroy } from "svelte";

    let value = "";
    const call_backend = async () => {
        await taurpc.update_state(value);
        await taurpc.get_window();
        await taurpc.method_with_alias();

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

    let unlisten = null;

    onMount(async () => {
        // unlisten = taurpc.update_state.on((new_state) => {
        //     console.log("state updated", new_state);
        // });
        unlisten = taurpc.ev.on((val) => {
            console.log("ev", val)
        })
    });

    onDestroy(() => {
        unlisten();
    });
</script>

<main class="container">
    Set managed state on backend <input type="text" bind:value />
    <button on:click={call_backend}>Call Backend code</button>
</main>
