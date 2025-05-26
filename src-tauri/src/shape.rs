use bytes::Bytes;
use serde::{Deserialize, Serialize};
use crate::drawable::Drawable;

#[derive(Debug, Serialize, Deserialize)]
pub struct ShapeCollection {
    #[serde(rename = "file-type")]
    pub file_type: String,
    pub shapes: Vec<Shape>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Shape {
    pub name: String,
    #[serde(rename = "type")]
    pub shape_type: String,
    pub representation: Vec<String>,
    #[serde(rename = "resize-width-indexes")]
    pub resize_width_indexes: Vec<ResizeIndex>,
    #[serde(rename = "resize-height-indexes")]
    pub resize_height_indexes: Vec<ResizeIndex>,
}

#[derive(Debug, Serialize, Deserialize)]
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
    
    fn render(&self) -> String {
        todo!()
    }
}