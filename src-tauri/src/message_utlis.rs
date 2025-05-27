
use crate::{ResultLoadAllShapeCollections, ShapeTemplateCollection, ShapeTemplate, ErrorInformation};
use crate::shape::ShapeCollection;

pub(crate) fn to_result_route_load_all_shape_collections(load_result : Result<Vec<ShapeCollection>, std::io::Error>) -> Result<ResultLoadAllShapeCollections, String> {
    match load_result {
        Ok(collections) => {
            let shape_collections = collections.into_iter().map(|col| {
                ShapeTemplateCollection {
                    name: col.name,
                    shapes: col.shapes.into_iter().map(|shape| {
                        ShapeTemplate {
                            id: shape.name.clone(), // Use a real unique id if available
                            name: shape.name,
                            prerender: shape.preview,
                        }
                    }).collect(),
                }
            }).collect();

            Ok(ResultLoadAllShapeCollections {
                shape_collections
            })
        }
        Err(e) => {
            Err(e.to_string())
        }
    }
}