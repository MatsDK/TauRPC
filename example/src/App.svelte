<script lang="ts">
    import { taurpc } from "./lib/ipc";
    import { onMount, onDestroy } from "svelte";

    let value = "";

    let state = "";
    const call_backend = async () => {
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
        unlisten.push(
            taurpc.events.vec_test.on((new_state) => {
                console.log("state updated", new_state);
            })
        );
        unlisten.push(
            taurpc.events.state_changed.on((val) => {
                state = val;
            })
        );
        unlisten.push(
            taurpc.events.multiple_args.on((arg1, arg2) => {
                console.log(arg1, arg2);
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

    <br />
    Current State (uppercase): {state}
</main>
