<script lang="ts">
    import { invoke } from "@tauri-apps/api";
    const click = () => {
        invoke("test_cmd");
    };
</script>

<main class="container">
    <h1>Test</h1>
    <button on:click={click}>Invoke</button>
</main>
