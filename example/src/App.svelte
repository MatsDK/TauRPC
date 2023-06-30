<script lang="ts">
    import { taurpc } from "./lib/rpc";

    let value = "";
    const call_backend = async () => {
        // const res = await taurpc.test_io({
        //     first_name: "foo",
        //     last_name: "bar",
        //     uid: 0,
        // });
        // console.log(res);

        await taurpc.update_state(value);
        // await taurpc.get_window();
        // await taurpc.get_app_handle();
        console.log("before sleep");
        await taurpc.with_sleep();
        console.log("after sleep");
    };
</script>

<main class="container">
    Set managed state on backend <input type="text" bind:value />
    <button on:click={call_backend}>Call Backend code</button>
</main>
