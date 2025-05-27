<script lang="ts">
    import {ResultLoadAllShapeCollections, ShapeTemplateCollection} from "../gen/shape_template.js";
    import {callAndReceiveAsync} from "../utils/communication.js";
    import {showError} from "$lib/stores/error";

    let shape_collections = $state<ShapeTemplateCollection[]>([]);
    async function load_all_shape_collections()
    {
        try {
            const res = await callAndReceiveAsync("route_load_all_shape_collections", ResultLoadAllShapeCollections);
            shape_collections = res.shapeCollections;
        }
        catch (e : unknown)
        {
            showError(String(e))
        }
    }
    load_all_shape_collections();
</script>

<div class="sidebar-templates">
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