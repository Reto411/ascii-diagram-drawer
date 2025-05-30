<script lang="ts">
    import {AllShapeCollectionsReloaded} from "../gen/events/AllShapeCollectionsReloaded";
    import { ShapeTemplateCollection } from "../gen/types/ShapeTemplate.js";
    import {ShapeCollectionLoaded} from "../gen/events/ShapeCollectionLoaded.js";
    import {callAndReceiveAsync, listenBackendEvent} from "../utils/communication.js";
    import {showError} from "$lib/stores/error";
    import {onMount} from "svelte";
    import {invoke} from "@tauri-apps/api/core";

    let shape_collections = $state<ShapeTemplateCollection[]>([]);

    onMount(() => {
        shape_collections = []
        listenBackendEvent('ShapeCollectionLoaded', ShapeCollectionLoaded, (event: ShapeCollectionLoaded) => {
            if (event.shapeCollection != undefined) {
                if (!shape_collections.some(c => c.name === event.shapeCollection!.name)) {
                    shape_collections.push(event.shapeCollection);
                }
            }
        });
        invoke("initialize");
    });
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