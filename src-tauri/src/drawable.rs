

pub trait Drawable{
    fn get_name(&self) -> String;
    fn draw(&self) -> String;
    fn save(&self);
    fn expand(&self);
    fn shrink(&self);
    fn pre_render(&self) -> Result<String, std::io::Error>;
}

/*
struct DiagramObject {
    name: String,
    x: u64,
    y: u64,
    representation: Vec<Vec<u64>>,
    image: bytes::Bytes,
}*/