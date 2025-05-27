
use crate::messages;
use crate::shape::ShapeCollection;

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