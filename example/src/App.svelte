<script lang="ts">
    import { taurpc } from "./lib/ipc";
    import { onMount, onDestroy } from "svelte";

    let value = "";
    const call_backend = async () => {
        await taurpc.events.test_ev();
        await taurpc.update_state(value);
        await taurpc.get_window();
        await taurpc.method_with_alias();
        await taurpc.multiple_args([], "test");

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

    let unlisten = [];

    onMount(async () => {
        // unlisten = taurpc.update_state.on((new_state) => {
        //     console.log("state updated", new_state);
        // });
        unlisten.push(
            taurpc.update_state.on((val) => {
                console.log("update_state", val);
            })
        );
        unlisten.push(
            taurpc.events.test_ev.on(() => {
                console.log("events.test_ev");
            })
        );
    });

    onDestroy(() => {
        unlisten.forEach((fn) => fn());
    });
</script>

<main class="container">
    Set managed state on backend <input type="text" bind:value />
    <button on:click={call_backend}>Call Backend code</button>
</main>
