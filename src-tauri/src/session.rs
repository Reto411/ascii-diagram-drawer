use crate::drawable::Drawable;
use crate::RESOURCE_PATH;
use crate::shape_template_collections::{ShapeTemplateCollection};

pub struct Session {
    pub loaded_shape_template_collections: Vec<ShapeTemplateCollection>,

}

impl Session {
    pub fn new() -> Session {
        Session {
            loaded_shape_template_collections: Vec::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), std::io::Error> {
        let _current_path = std::fs::read_dir("./")?;
        for entry in std::fs::read_dir(RESOURCE_PATH)? {
            let file = entry?.path();
            // ignore dirs
            if file.is_dir() {
                continue;
            }

            // Ignore files that have "schema" in their filename
            if let Some(filename) = file.file_name().and_then(|n| n.to_str()) {
                if filename.contains("schema") {
                    continue;
                }
            }
            self.loaded_shape_template_collections.push(ShapeTemplateCollection::new(file));
        }
        Ok(())
    }
}