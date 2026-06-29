<script lang="ts">
  import { onMount } from 'svelte'
  import { createTauRPCProxy } from './lib/ipc'

  const taurpc = createTauRPCProxy()
  
  let stringStatus = "Waiting..."
  let enumStatus = "Waiting..."
  let structStatus = "Waiting..."
  let thiserrorStatus = "Waiting..."
  let channelStatus = "Waiting..."
  let channelUpdates: string[] = []

  const testStringError = async (fail: boolean) => {
    stringStatus = "Loading..."
    try {
      const res = await taurpc.error_testing.test_string_error(fail)
      stringStatus = res.status === 'error' 
        ? `Backend Error: ${res.error}` 
        : `Success: ${res.data}`
    } catch (e) {
      stringStatus = `Exception: ${e}`
    }
  }

  const testEnumError = async (fail: boolean) => {
    enumStatus = "Loading..."
    try {
      const res = await taurpc.error_testing.test_enum_error(fail)
      if (res.status === 'error') {
        // Here TypeScript knows res.error is of type CustomError (SimpleError | MessageError | ComplexError)
        enumStatus = `Backend Error: ${JSON.stringify(res.error)}`
      } else {
        // TypeScript knows res.data is ComplexData
        enumStatus = `Success: ID ${res.data.id}, Payload: ${res.data.payload}`
      }
    } catch (e) {
      enumStatus = `Exception: ${e}`
    }
  }

  const testStructError = async (fail: boolean) => {
    structStatus = "Loading..."
    try {
      const res = await taurpc.error_testing.test_struct_error(fail)
      structStatus = res.status === 'error'
        ? `Backend Error: Status ${res.error.status}, Message: ${res.error.message}`
        : `Success: void returned`
    } catch (e) {
      structStatus = `Exception: ${e}`
    }
  }

  const testThiserrorError = async (fail: boolean) => {
    thiserrorStatus = "Loading..."
    try {
      const res = await taurpc.error_testing.test_thiserror_error(fail)
      thiserrorStatus = res.status === 'error'
        ? `Backend Error (thiserror stringified): ${res.error}`
        : `Success: void returned`
    } catch (e) {
      thiserrorStatus = `Exception: ${e}`
    }
  }

  const testChannelError = async (fail: boolean) => {
    channelStatus = "Loading..."
    channelUpdates = []
    
    try {
      const res = await taurpc.error_testing.test_with_channel(fail, (msg) => {
        channelUpdates = [...channelUpdates, msg]
      })
      channelStatus = res.status === 'error'
        ? `Backend Error: ${JSON.stringify(res.error)}`
        : `Success: Process completed`
    } catch (e) {
      channelStatus = `Exception: ${e}`
    }
  }
</script>

<div class="error-testing">
  <h2>Result Mode Edge Cases</h2>
  
  <div class="test-block">
    <h3>String Error</h3>
    <button on:click={() => testStringError(false)}>Test Success</button>
    <button on:click={() => testStringError(true)}>Test Fail</button>
    <p>Result: <code>{stringStatus}</code></p>
  </div>

  <div class="test-block">
    <h3>Enum Error (CustomError)</h3>
    <button on:click={() => testEnumError(false)}>Test Success</button>
    <button on:click={() => testEnumError(true)}>Test Fail</button>
    <p>Result: <code>{enumStatus}</code></p>
  </div>

  <div class="test-block">
    <h3>Struct Error (StructError)</h3>
    <button on:click={() => testStructError(false)}>Test Success</button>
    <button on:click={() => testStructError(true)}>Test Fail</button>
    <p>Result: <code>{structStatus}</code></p>
  </div>

  <div class="test-block">
    <h3>ThisError (Serialized as String)</h3>
    <button on:click={() => testThiserrorError(false)}>Test Success</button>
    <button on:click={() => testThiserrorError(true)}>Test Fail</button>
    <p>Result: <code>{thiserrorStatus}</code></p>
  </div>

  <div class="test-block">
    <h3>Result with Channel argument</h3>
    <button on:click={() => testChannelError(false)}>Test Success</button>
    <button on:click={() => testChannelError(true)}>Test Fail</button>
    <p>Result: <code>{channelStatus}</code></p>
    {#if channelUpdates.length > 0}
      <ul>
        {#each channelUpdates as update}
          <li>{update}</li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<style>
  .error-testing {
    border: 1px solid #ccc;
    padding: 1.5rem;
    margin-top: 2rem;
    border-radius: 8px;
    background-color: #fcfcfc;
    box-shadow: 0 2px 4px rgba(0,0,0,0.05);
  }
  .test-block {
    margin-top: 1rem;
    padding: 1rem;
    background-color: #fff;
    border: 1px solid #eee;
    border-radius: 4px;
  }
  button {
    background-color: #2196F3;
    color: white;
    border: none;
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-weight: bold;
  }
  button:hover {
    background-color: #1976D2;
  }
  code {
    background: #eef;
    padding: 2px 4px;
    border-radius: 4px;
    color: #333;
  }
</style>
