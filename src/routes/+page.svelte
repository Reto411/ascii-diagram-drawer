<script>
    import {invoke} from "@tauri-apps/api/core";

    let name = $state("");
    let greetMsg = $state("");

    async function greet(event) {
        event.preventDefault();
        // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
        greetMsg = await invoke("greet", {name});
    }
</script>

<main class="main-content">
    <div class="sidebar">
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
    <div class="editor">
        Editor
    </div>
    <p>{greetMsg}</p>
</main>
