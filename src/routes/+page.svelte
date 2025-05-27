<script lang="ts">
    import {invoke} from "@tauri-apps/api/core";
    import {ResultLoadAllShapeCollections, ShapeTemplateCollection} from "../gen/shape_template";
    import {callAndReceiveAsync} from "../utils/communication";

    let sidebarWidth = $state(260);
    let isResizing = $state(false);
    let gridSize = $state(32);
    let offsetX = $state(0);
    let offsetY = $state(0);
    let isPanning = $state(false);
    let lastX =  $state(0.0)
    let lastY = $state(0.0);

    function handleWheel(event : WheelEvent) {
        if (event.ctrlKey) {
            event.preventDefault();
            const oldGridSize = gridSize;
            if (event.deltaY < 0) {
                gridSize = Math.min(gridSize * 1.1, 128);
            } else {
                gridSize = Math.max(gridSize / 1.1, 8);
            }
            // Optional: Zoom to mouse position (advanced)
        } else if (isPanning) {
            // Prevent default scroll while panning
            event.preventDefault();
        }
    }

    function handleMouseDown(event : MouseEvent) {
        if (event.button === 1) { // Middle mouse button
            event.preventDefault();
            isPanning = true;
            lastX = event.clientX;
            lastY = event.clientY;
            window.addEventListener('mousemove', handleMouseMove);
            window.addEventListener('mouseup', handleMouseUp);
            document.body.style.cursor = 'grab';
        }

        function handleMouseMove(event : MouseEvent) {
            if (isPanning) {
                offsetX += event.clientX - lastX;
                offsetY += event.clientY - lastY;
                lastX = event.clientX;
                lastY = event.clientY;
            }
        }

        function handleMouseUp(event : MouseEvent) {
            if (event.button === 1) {
                isPanning = false;
                window.removeEventListener('mousemove', handleMouseMove);
                window.removeEventListener('mouseup', handleMouseUp);
                document.body.style.cursor = '';
            }
        }
    }

    function startResize() {
        document.body.classList.add('noselect');
        isResizing = true;
        document.body.style.cursor = 'col-resize';

        function onMouseMove(e) {
            sidebarWidth = Math.max(120, Math.min(window.innerWidth * 0.5, e.clientX));
        }

        function onMouseUp() {
            document.body.classList.remove('noselect');
            isResizing = false;
            document.body.style.cursor = '';
            window.removeEventListener('mousemove', onMouseMove);
            window.removeEventListener('mouseup', onMouseUp);
        }

        window.addEventListener('mousemove', onMouseMove);
        window.addEventListener('mouseup', onMouseUp);
    }

    function zoomIn() {
        gridSize = Math.min(gridSize * 1.25, 128);
    }
    function zoomOut() {
        gridSize = Math.max(gridSize / 1.25, 8);
    }

    function base64ToUint8Array(base64: string): Uint8Array {
        const binary = atob(base64);
        const len = binary.length;
        const bytes = new Uint8Array(len);
        for (let i = 0; i < len; i++) {
            bytes[i] = binary.charCodeAt(i);
        }
        return bytes;
    }

    let errors = $state<String[]>([]);
    let shape_collections = $state<ShapeTemplateCollection[]>([]);
    async function load_all_shape_collections()
    {
        try {
            const res = await callAndReceiveAsync("route_load_all_shape_collections", ResultLoadAllShapeCollections);
            shape_collections = res.shapeCollections;
        }
        catch (e : unknown)
        {
            errors.push(String(e))
        }
    }
    load_all_shape_collections();
</script>

{#if errors && errors.length > 0 }
    <div class="errors">
            <ul>
                {#each errors as error }
                    <li>{error}</li>
                {/each}
            </ul>
    </div>
{/if}
<main class="main-content">
    <div class="sidebar" style="width: {sidebarWidth}px !important;">
        <div class="sidebar-category" style="border-bottom: 5px #333333; margin: 0;">
        Templates
        </div>
        {#each shape_collections as collection}
            <div class="sidebar-category">
                { collection.name }
            </div>
            {#each collection.shapes as shape }
                <div class="sidebar-item">
                    {@html shape.prerender }
                    <p>{shape.name}</p>
                </div>
            {/each}
        {/each}
    </div>
    <div class="sidebar-resizer" role="presentation"  onmousedown={startResize}></div>
    <div class="editor">
        <div class="editor-toolbar">
            <button onclick={zoomOut}>-</button>
            <span>Grid: {Math.round(gridSize)}px</span>
            <button onclick={zoomIn}>+</button>
        </div>
        <div role="presentation" onwheel="{handleWheel}" onmousedown={handleMouseDown} class="editor-grid" style="
             --grid-size: {gridSize}px; background-size: var(--grid-size) var(--grid-size);">
        </div>
    </div>
</main>
