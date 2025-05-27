<script lang="ts">
    import {writable} from "svelte/store";
    import {errorBar, ErrorBarType} from "$lib/stores/error";

</script>

{#if $errorBar.visible}
    <div class="error-bar { $errorBar.type }">
        <span class="icon">
            {#if $errorBar.type === ErrorBarType.Error}
                &#9888; <!-- warning icon -->
            {:else if $errorBar.type === ErrorBarType.Info}
                &#8505; <!-- info icon -->
            {:else if $errorBar.type === ErrorBarType.Success}
                &#10003; <!-- checkmark icon -->
            {/if}
            <span class="message">{$errorBar.message}</span>
        </span>
    </div>
{/if}

<style>
.error-bar {
    display: flex;
    align-items: center;
    gap: 0.75em;
    padding: 1em 1.5em;
    margin: 0 0 0 0;
    box-shadow: 0 2px 12px rgba(0,0,0,0.08);
    font-size: 1.1em;
    font-weight: 500;
    width: 100%;
    position: relative;
    animation: slideDown 0.3s cubic-bezier(.4,0,.2,1);
    border-left: 6px solid;
    background: #fff;
}
.error-bar.error {
    border-color: #e74c3c;
    color: #a94442;
    background: #fdecea;
}
.error-bar.info {
    border-color: #3498db;
    color: #1e5a7a;
    background: #eaf6fd;
}
.error-bar.success {
    border-color: #2ecc71;
    color: #1d643b;
    background: #eafaf1;
}
.icon {
    font-size: 1.5em;
    margin-right: 0.5em;
    flex-shrink: 0;
}
@keyframes slideDown {
    from { transform: translateY(-20px); opacity: 0; }
    to   { transform: translateY(0); opacity: 1; }
}
</style>