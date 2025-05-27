<script lang="ts">
    import TemplateSidebar from "$lib/template_sidebar.svelte";
    import DiagramEditor from "$lib/diagram_editor.svelte";

    let sidebarWidth = $state(260);

    function startResize() {
        document.body.classList.add('noselect');
        document.body.style.cursor = 'col-resize';

        function onMouseMove(e : MouseEvent) {
            sidebarWidth = Math.max(120, Math.min(window.innerWidth * 0.5, e.clientX));
        }

        function onMouseUp() {
            document.body.classList.remove('noselect');
            document.body.style.cursor = '';
            window.removeEventListener('mousemove', onMouseMove);
            window.removeEventListener('mouseup', onMouseUp);
        }

        window.addEventListener('mousemove', onMouseMove);
        window.addEventListener('mouseup', onMouseUp);
    }


</script>

<main class="main-content">
    <div class="sidebar" style="width: {sidebarWidth}px !important;">
        <TemplateSidebar></TemplateSidebar>
    </div>
    <div class="sidebar-resizer" role="presentation"  onmousedown={startResize}></div>
    <div class="editor">
        <DiagramEditor></DiagramEditor>
    </div>
</main>
