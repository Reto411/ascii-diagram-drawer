<script>
    import {invoke} from "@tauri-apps/api/core";

    let sidebarWidth = $state(260);
    let isResizing = false;

    let gridSize = $state(32);
    let offsetX = $state(0);
    let offsetY = $state(0);
    let isPanning = $state(false);
    let lastX, lastY;

    let name = $state("");
    let greetMsg = $state("");

    function handleWheel(event) {
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

    function handleMouseDown(event) {
        if (event.button === 1) { // Middle mouse button
            event.preventDefault();
            isPanning = true;
            lastX = event.clientX;
            lastY = event.clientY;
            window.addEventListener('mousemove', handleMouseMove);
            window.addEventListener('mouseup', handleMouseUp);
            document.body.style.cursor = 'grab';
        }

        function handleMouseMove(event) {
            if (isPanning) {
                offsetX += event.clientX - lastX;
                offsetY += event.clientY - lastY;
                lastX = event.clientX;
                lastY = event.clientY;
            }
        }

        function handleMouseUp(event) {
            if (event.button === 1) {
                isPanning = false;
                window.removeEventListener('mousemove', handleMouseMove);
                window.removeEventListener('mouseup', handleMouseUp);
                document.body.style.cursor = '';
            }
        }
    }

    function startResize(e) {
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

    async function greet(event) {
        event.preventDefault();
        // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
        greetMsg = await invoke("greet", {name});
    }
</script>

<main class="main-content">
    <div class="sidebar" style="width: {sidebarWidth}px !important;">
        <div class="sidebar-category" style="border-bottom: 5px #333333; margin: 0;">
        Templates
        </div>
        <div>
            <div class="sidebar-category">
                Class 1
            </div>
            <div class="sidebar-item">
                <img src="/box.png" alt="the image to insert">
                <p>Box</p>
            </div>
            <div class="sidebar-item">
                <img src="/box.png" alt="the image to insert">
                <p>Box</p>
            </div>
            <div class="sidebar-item">
                <img src="/box.png" alt="the image to insert">
                <p>Box</p>
            </div>
            <div class="sidebar-item">
                <img src="/box.png" alt="the image to insert">
                <p>Box</p>
            </div>
        </div>
        <div>
            <div class="sidebar-category">
                Class 2
            </div>
            <div class="sidebar-item">
                <img src="/box.png" alt="the image to insert">
                <p>Box</p>
            </div>
            <div class="sidebar-item">
                <img src="/box.png" alt="the image to insert">
                <p>Box</p>
            </div>
            <div class="sidebar-item">
                <img src="/box.png" alt="the image to insert">
                <p>Box</p>
            </div>
            <div class="sidebar-item">
                <img src="/box.png" alt="the image to insert">
                <p>Box</p>
            </div>
        </div>
    </div>
    <div class="sidebar-resizer" aria-orientation="vertical" role="separator"  onmousedown={startResize}></div>
    <div class="editor">
        <div class="editor-toolbar">
            <button onclick={zoomOut}>-</button>
            <span>Grid: {Math.round(gridSize)}px</span>
            <button onclick={zoomIn}>+</button>
        </div>
        <div onwheel="{handleWheel}" onmousedown={handleMouseDown} class="editor-grid" style="
             --grid-size: {gridSize}px; background-size: var(--grid-size) var(--grid-size);">
        </div>
    </div>
    <p>{greetMsg}</p>
</main>
