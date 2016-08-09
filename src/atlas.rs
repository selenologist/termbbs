use std;
use rustc_serialize;
use image;
use glium;
use std::fs::File;

#[derive(Copy,Clone)]
pub struct AtlasDimensions{
    /* The following data is needed both as integer and as float.
     * Conversion between the two at runtime can be slow, so keep it in both
     * formats. Shouldn't actually matter to performance though.
    */

    pub atlas_w_u: u16,
    pub atlas_h_u: u16,
    pub tile_w_u:  u16,
    pub tile_h_u:  u16,
    pub atlas_w_f: f32,
    pub atlas_h_f: f32,
    pub tile_w_f:  f32,
    pub tile_h_f:  f32
}

impl AtlasDimensions{
    fn new_from_u16(atlas_width: u16, atlas_height: u16,
                    tile_width: u16, tile_height: u16) -> AtlasDimensions{
        AtlasDimensions {
            atlas_w_u: atlas_width,
            atlas_w_f: atlas_width  as f32,
            atlas_h_u: atlas_height,
            atlas_h_f: atlas_height as f32,
            tile_w_u: tile_width,
            tile_w_f: tile_width    as f32,
            tile_h_u: tile_height,
            tile_h_f: tile_height   as f32,
        }
    }
/*   fn new_from_f32(atlas_width: f32, atlas_height: f32,
                    tile_width: f32, tile_height: f32) -> AtlasDimensions{
        AtlasDimensions {
            atlas_w_u: atlas_width  as u16,
            atlas_w_f: atlas_width,
            atlas_h_u: atlas_height as u16,
            atlas_h_f: atlas_height,
            tile_w_u: tile_width    as u16,
            tile_w_f: tile_width,
            tile_h_u: tile_height   as u16,
            tile_h_f: tile_height,
        }
    } */
}

#[derive(Debug, PartialEq, RustcDecodable, RustcEncodable)]
struct AtlasDescriptor{
    /* Serde uses JavaScript types, so the width of integers is 64 bits
       it seems to accept smaller types but I don't know what it does when it
       overflows so let's be defensive and handle it ourselves. */
    tile_width:  i64,
    tile_height: i64,
    atlas_path:  String,
    tile_labels: Option<Vec<String>>
}

#[derive(Debug)]
pub enum AtlasErr{
    Io(std::io::Error),
    Parse(rustc_serialize::json::DecoderError),
    Image(image::ImageError),
    IntegerRange
}

pub struct Atlas{
    pub dimensions: AtlasDimensions,
    pub texture:    glium::texture::Texture2d,
    pub labels:     Vec<String>
}

use std::path::Path;
use std::path::PathBuf;
impl Atlas{
    pub fn new_from_file_blocking<F: glium::backend::Facade>
        (glium: &F, path: &Path) -> Result<Atlas, AtlasErr>
    {
        use std::io::Read;
        let descriptor:AtlasDescriptor =
            match rustc_serialize::json::decode(&mut
                                          match File::open(path){
                                              Ok(mut f)  => {let mut s: String = String::new();
                                                         match f.read_to_string(&mut s){
                                                             Ok(_)  => s,
                                                             Err(e) => return Err(AtlasErr::Io(e))
                                                         }
                                              },
                                              Err(e) => return Err(AtlasErr::Io(e))
                                          }){
                Ok(ok) => ok,
                Err(e) => return Err(AtlasErr::Parse(e))
            };

        if     descriptor.tile_width  < std::u16::MIN as i64 || descriptor.tile_width  > std::u16::MAX as i64
            || descriptor.tile_height < std::u16::MIN as i64 || descriptor.tile_height > std::u16::MAX as i64
        {
            return Err(AtlasErr::IntegerRange);
        }

        let mut image_pathbuf = PathBuf::from(path);
        image_pathbuf.pop();
        image_pathbuf.push(&descriptor.atlas_path); // SEC: consider directory traversal
        let image_path    = image_pathbuf.as_path();
        let image = match image::open(&image_path){
            Ok(ok) => ok.to_rgba(),
            Err(e) => return Err(AtlasErr::Image(e))
        };
        let image_dimensions = image.dimensions();
        let image = glium::texture::RawImage2d::from_raw_rgba_reversed(image.into_raw(), image_dimensions);
        let texture = glium::texture::Texture2d::new(glium, image).expect("Failed to get glium texture for atlas");
        Ok(Atlas {
            dimensions: AtlasDimensions::new_from_u16(image_dimensions.0 as u16,
                                                      image_dimensions.1 as u16,
                                                      descriptor.tile_width as u16,
                                                      descriptor.tile_height as u16),
            texture:    texture,
            labels: match descriptor.tile_labels {
                Some(l) => l,
                None    => Vec::new()
            }
        })
    }
}
