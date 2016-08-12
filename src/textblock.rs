use atlas;
use glium;

use atlas::Atlas;
use profiling_timers::ScopeTimer;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    texcoord: [f32; 2]
}
implement_vertex!(Vertex, position, texcoord);

pub struct TextBlock{
    atlas:  atlas::AtlasDimensions,
    width:  u32,
    height: u32,
    block:  Vec<u16>,
    tiles:  glium::texture::UnsignedTexture1d,
    vbo:    glium::VertexBuffer<Vertex>,
    ibo:    glium::index::IndexBuffer<u16>,
    scanline_y: f32,
}

#[derive(Debug)]
pub enum TextBlockErr{
    WrongSizeBlock,
    VBOCreation(glium::vertex::BufferCreationError),
    IBOCreation(glium::index::BufferCreationError),
    TextureCreation(glium::texture::TextureCreationError)
}

impl TextBlock{
    fn generate_tile_triangles(atlas: &atlas::AtlasDimensions,
                               position: [f32; 2]) -> [Vertex; 4]{
        let atlas_columns:u32 = (atlas.atlas_w_u / atlas.tile_w_u) as u32;

        enum Corner{
            TL, TR, BL, BR
        }
        impl Corner{
            fn to_offset(self) -> (f32, f32){
                match self{
                    Corner::TL => (0f32, 1f32),
                    Corner::TR => (1f32, 1f32),
                    Corner::BL => (0f32, 0f32),
                    Corner::BR => (1f32, 0f32)
                }
            }
        }
        let get_pos_coord = |corner: Corner| -> [f32; 2]{
            let (addx, addy) = corner.to_offset();
            [ (position[0] + addx),
             -(position[1] + addy)]
        };
        let get_atlas_coord = |corner: Corner| -> [f32; 2]{
            let (addx, addy) = corner.to_offset();
            [         atlas.tile_w_f / atlas.atlas_w_f * addx,
             1.0f32 - atlas.tile_h_f / atlas.atlas_h_f * addy]
        };

        [Vertex{position: get_pos_coord  (Corner::TL),
                texcoord: get_atlas_coord(Corner::TL)}, // 0
         Vertex{position: get_pos_coord  (Corner::BL),
                texcoord: get_atlas_coord(Corner::BL)}, // 1
         Vertex{position: get_pos_coord  (Corner::TR),
                texcoord: get_atlas_coord(Corner::TR)}, // 2
         Vertex{position: get_pos_coord  (Corner::BR),
                texcoord: get_atlas_coord(Corner::BR)}, // 3
        ]
    }

    fn generate_tile_indices(tile_index: u32) -> [u16; 6]{
        let index_base = tile_index * 4;

        [(index_base + 0u32) as u16, (index_base + 1) as u16, (index_base + 2) as u16,
         (index_base + 2)    as u16, (index_base + 1) as u16, (index_base + 3) as u16]
    }

    #[allow(unused_variables)]
    fn update<F>(&mut self, glium: &F) where F: glium::backend::Facade
    {
        let outer  = ScopeTimer::new("tile-based update");
        self.tiles = glium::texture::UnsignedTexture1d::new(glium, self.block.clone())
            .expect("Failed to make texture");
    }

    pub fn new<F>(glium: &F, atlas: &Atlas,
                  width: u32, height: u32, block: Option<&[u16]>)
                  -> Result<TextBlock, TextBlockErr>
        where F: glium::backend::Facade{
        let final_block = match block{
            Some(x) => {
                if x.len() as u32 != (width * height){
                    return Err(TextBlockErr::WrongSizeBlock);
                }
                else{
                    x.to_vec()
                }
            },
            None    => vec![0u16; (width * height) as usize]
        };

        let mut triangles:Vec<Vertex> =
            Vec::with_capacity((height * width * 4) as usize); // 4 unique vertices per tile

        let mut indices:Vec<u16> =
            Vec::<u16>::with_capacity((height * width * 6) as usize); // 6 indexed vertices per tile

        for index in 0..(width * height) as u32{
            triangles.extend_from_slice(
                &TextBlock::generate_tile_triangles(
                    &atlas.dimensions,
                    [(index % width) as f32,
                     (index / width) as f32]));

            indices.extend_from_slice(
                &TextBlock::generate_tile_indices(
                    index as u32))
        }

        let vbo =
            match glium::VertexBuffer::persistent(glium,
                                                  &triangles)
        {
            Ok(v)  => v,
            Err(e) => return Err(TextBlockErr::VBOCreation(e))
        };

        let ibo =
            match glium::IndexBuffer::persistent(glium,
                                                 glium::index::PrimitiveType::TrianglesList,
                                                 &indices)
        {
            Ok(i)  => i,
            Err(e) => return Err(TextBlockErr::IBOCreation(e))
        };

        let tiles =
            match glium::texture::UnsignedTexture1d::new(glium, final_block.clone())
        {
            Ok(t)  => t,
            Err(e) => return Err(TextBlockErr::TextureCreation(e))
        };

        let tb = TextBlock{
            atlas:  atlas.dimensions.clone(),
            width:  width,
            height: height,
            block:  final_block,
            tiles:  tiles,
            vbo:    vbo,
            ibo:    ibo,
            scanline_y: 0.0f32
        };

        Ok(tb)
    }

    pub fn draw<F>(&mut self,
                display: &F,
                program: &glium::Program,
                target: &mut glium::Frame,
                atlas: &Atlas) where F: glium::backend::Facade{
        use glium::Surface;
        use nalgebra::*;

        let (scale_x, scale_y) =
            (2.0f32 / (self.width  as f32),
             2.0f32 / (self.height as f32));

        let scaled_matrix: Matrix3<f32> =
            Matrix3::new(scale_x, 0.0f32,  0.0f32,
                         0.0f32,  scale_y, 0.0f32,
                         0.0f32,  0.0f32,  1.0f32);

        let matrix: Matrix3<f32> = // position in top left corner
            Matrix3::new(1.0f32, 0.0f32,-1.0f32,
                         0.0f32, 1.0f32, 1.0f32,
                         0.0f32, 0.0f32, 1.0f32) * scaled_matrix;

        self.update(display);

        let uniforms = uniform! {
            tex:           &atlas.texture,
            matrix:        *matrix.as_ref(),
            scanline_y:    self.scanline_y,
            tile_id:      &self.tiles,
            tile_width:    self.atlas.tile_w_f / self.atlas.atlas_w_f,
            tile_height:   self.atlas.tile_h_f / self.atlas.atlas_h_f,
            atlas_columns: (self.atlas.atlas_w_u / self.atlas.tile_w_u) as u32
        };

        target.draw(&self.vbo, &self.ibo, program, &uniforms,
                    &Default::default()).expect("Failed to draw");

        self.scanline_y += 1.0f32 / 5.0f32 + 0.02f32; // this is just some random number tbh. Should appear like multiple bars crawling up the screen.
        if self.scanline_y > 1.2{   // if greater than the size of the screen and a little bit
            self.scanline_y -= 1.4; // set the scanline to just before the screen by a little bit (-0.4)
        }
    }
}
