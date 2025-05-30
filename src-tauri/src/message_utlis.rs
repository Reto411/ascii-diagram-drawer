
use crate::messages;
use crate::messages::events::BackendErrorOccurred;
use crate::shape_template_collections::ShapeCollection;


pub(crate) fn clone_to_event_message_collection_loaded(shape_collection: &ShapeCollection) -> messages::events::ShapeCollectionLoaded {
    let shape_templates = shape_collection.shapes.iter().map(|shape| {
        messages::types::ShapeTemplate {
            id: shape.name.clone(), // Use a real unique id if available
            name: shape.name.clone(),
            prerender: shape.preview.clone(),
        }
    }).collect();

    let proto_collection = messages::types::ShapeTemplateCollection {
        name: shape_collection.name.clone(),
        shapes: shape_templates,
    };

    messages::events::ShapeCollectionLoaded {
        shape_collection: Some(proto_collection),
    }
}

pub(crate) fn clone_io_error_to_backend_error_occurred(error_message: String) -> messages::events::BackendErrorOccurred {
    BackendErrorOccurred{
        error_message: error_message,
    }
}

pub(crate) fn to_result_route_load_all_shape_collections(load_result : Result<Vec<ShapeCollection>, std::io::Error>) -> Result<messages::events::AllShapeCollectionsReloaded, String> {
    match load_result {
        Ok(collections) => {
            let shape_collections = collections.into_iter().map(|col| {
                messages::types::ShapeTemplateCollection {
                    name: col.name,
                    shapes: col.shapes.into_iter().map(|shape| {
                        messages::types::ShapeTemplate {
                            id: shape.name.clone(), // Use a real unique id if available
                            name: shape.name,
                            prerender: shape.preview,
                        }
                    }).collect(),
                }
            }).collect();

            Ok(messages::events::AllShapeCollectionsReloaded {
                shape_collections
            })
        }
        Err(e) => {
            Err(e.to_string())
        }
    }
}