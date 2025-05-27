<script lang="ts">
    let offsetX = $state(0);
    let offsetY = $state(0);
    let isPanning = $state(false);
    let lastX =  $state(0.0)
    let lastY = $state(0.0);
    let gridSize = $state(32);

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

    function zoomIn() {
        gridSize = Math.min(gridSize * 1.25, 128);
    }
    function zoomOut() {
        gridSize = Math.max(gridSize / 1.25, 8);
    }
</script>

<div class="editor-toolbar">
    <button onclick={zoomOut}>-</button>
    <span>Grid: {Math.round(gridSize)}px</span>
    <button onclick={zoomIn}>+</button>
</div>
<div role="presentation" onwheel="{handleWheel}" onmousedown={handleMouseDown} class="editor-grid" style="
             --grid-size: {gridSize}px; background-size: var(--grid-size) var(--grid-size);">
</div>