use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::drawable::Drawable;
use svgbob;
use crate::{shape_template_collections, RESOURCE_PATH};


pub struct ShapeTemplateCollection {
    pub file_path : std::path::PathBuf,
    pub shape_collection: Option<ShapeCollection>
}

impl ShapeTemplateCollection {
    pub fn new(file_path : std::path::PathBuf) -> ShapeTemplateCollection {
        ShapeTemplateCollection {
            shape_collection : None,
            file_path,
        }
    }
    
    pub async fn get_shape_collection(&mut self) -> Result<ShapeCollection, std::io::Error> {
        if self.shape_collection.is_none() {
            let reader = std::fs::File::open(self.file_path.clone())?;
            self.shape_collection = Some(serde_json::from_reader(reader)?);
            
            if self.shape_collection.is_some() {
                self.shape_collection.as_mut().unwrap().pre_render_all()?;
            }
        }
        Ok(self.shape_collection.clone().unwrap())
    }
}

pub async fn load_all_shape_collections() -> Result<Vec<shape_template_collections::ShapeCollection>, std::io::Error> {
    let mut collections = Vec::new();
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
        let reader = std::fs::File::open(file)?;
        let mut collection : ShapeCollection = serde_json::from_reader(reader)?;
        for mut item in &mut collection.shapes {
            item.preview = item.pre_render()?;
        }
        collections.push(collection);
    }
    Ok(collections)
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShapeCollection {
    #[serde(rename = "file-type")]
    pub file_type: String,
    pub name : String,
    pub shapes: Vec<Shape>,
}

impl ShapeCollection {
    pub fn pre_render_all(&mut self) -> Result<(),std::io::Error> {
        for mut shape in &mut self.shapes {
            shape.preview = shape.pre_render()?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Shape {
    pub name: String,
    #[serde(rename = "type")]
    pub shape_type: String,
    #[serde(rename = "representation-filepath")]
    pub representation_filepath: String,
    #[serde(rename = "resize-width-indexes")]
    pub resize_width_indexes: Vec<ResizeIndex>,
    #[serde(rename = "resize-height-indexes")]
    pub resize_height_indexes: Vec<ResizeIndex>,
    #[serde(default)]
    pub preview: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResizeIndex {
    pub x: i32,
    pub y: i32,
    pub character: String,
}

impl Drawable for Shape {
    fn get_name(&self) -> String {
        todo!()
    }

    fn draw(&self) -> String {
        todo!()
    }

    fn save(&self) {
        todo!()
    }

    fn expand(&self) {
        todo!()
    }
    
    fn shrink(&self) {
        todo!()
    }
    
    fn pre_render(&self) -> Result<String, std::io::Error> {
        let path = Path::new(RESOURCE_PATH).join(&self.representation_filepath);
        let representation = fs::read_to_string(path)?;
        Ok(svgbob::to_svg(representation.as_str()))
    }
}