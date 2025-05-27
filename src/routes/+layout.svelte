<script lang="ts">
    import ErrorBar from '$lib/error_bar.svelte';
    import NavBar from '$lib/nav_bar.svelte';
</script>


<NavBar />
<ErrorBar />
<slot  />
<footer class="footer">
    <div class="container-fluid bg-light">
        Ascii Diagrams Drawer
    </div>
</footer>
