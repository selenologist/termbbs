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
    block:  Vec<u8>,
    vbo:    glium::VertexBuffer<Vertex>,
    ibo:    glium::index::IndexBuffer<u16>,
    scanline_y: f32,
}

#[derive(Debug)]
pub enum TextBlockErr{
    WrongSizeBlock,
    VBOCreation(glium::vertex::BufferCreationError),
    IBOCreation(glium::index::BufferCreationError)
}

impl TextBlock{
    fn generate_tile_triangles(&self,
                               position: [f32; 2],
                               tile_id: u32) -> [Vertex; 4]{
        let atlas = self.atlas;
        let atlas_columns:u32 = (atlas.atlas_w_u / atlas.tile_w_u) as u32;

        enum Corner{
            TL, TR, BL, BR
        }
        let tile_pos_to_gl_pos = |ppos: [f32; 2]| -> [f32; 2]{
            [ ppos[0],
             -ppos[1]]
        };
        let get_atlas_coord = |corner: Corner| -> [f32; 2]{
            let (addx, addy) =
                match corner{
                    Corner::TL => (0, 1),
                    Corner::TR => (1, 1),
                    Corner::BL => (0, 0),
                    Corner::BR => (1, 0)
                };
            [         atlas.tile_w_f / atlas.atlas_w_f * (tile_id % atlas_columns + addx) as f32,
             1.0f32 - atlas.tile_h_f / atlas.atlas_h_f * (tile_id / atlas_columns + addy) as f32]
        };

        [Vertex{position: tile_pos_to_gl_pos([position[0] + 0.0f32,
                                              position[1] + 1.0f32]),
                texcoord: get_atlas_coord(Corner::TL)}, // 0
         Vertex{position: tile_pos_to_gl_pos([position[0] + 0.0f32,
                                              position[1] + 0.0f32]),
                texcoord: get_atlas_coord(Corner::BL)}, // 1
         Vertex{position: tile_pos_to_gl_pos([position[0] + 1.0f32,
                                              position[1] + 1.0f32]),
                texcoord: get_atlas_coord(Corner::TR)}, // 2
         Vertex{position: tile_pos_to_gl_pos([position[0] + 1.0f32,
                                              position[1] + 0.0f32]),
                texcoord: get_atlas_coord(Corner::BR)}, // 3
        ]
    }

    fn generate_tile_indices(tile_index: u32) -> [u16; 6]{
        let index_base = tile_index * 4;

        [(index_base + 0u32) as u16, (index_base + 1) as u16, (index_base + 2) as u16,
         (index_base + 2)    as u16, (index_base + 1) as u16, (index_base + 3) as u16]
    }

    #[allow(unused_variables)]
    fn update(&self){
        let outer = ScopeTimer::new("triangle-based update");

        let mut triangles:Vec<Vertex>;
        {
            let generation_timer = ScopeTimer::new("vtex array generation");
            triangles = Vec::with_capacity((self.height * self.width * 4) as usize); // 4 unique vertices per tile
            for (_index, _tile_id) in self.block.iter().enumerate(){
                let index   =  _index as u32;
                let tile_id = *_tile_id as u32;
                triangles.extend_from_slice(
                    &self.generate_tile_triangles(
                                        [(index % self.width) as f32,
                                         (index / self.width) as f32],
                        tile_id));
            }
        }

        {
            let upload_timer = ScopeTimer::new("GPU upload");
            self.vbo.write(&triangles);
        }
    }

    pub fn new<F>(glium: &F, atlas: &Atlas,
                  width: u32, height: u32, block: Option<&[u8]>)
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
            None    => vec![0u8; (width * height) as usize]
        };

        let vbo =
            match glium::VertexBuffer::empty_dynamic(glium,
                                                     (width * height * 4) as usize)
                                                     // 4 vertices per tile
        {
            Ok(v)  => v,
            Err(e) => return Err(TextBlockErr::VBOCreation(e))
        };

        let indices:Vec<u16> =
            (0..(width * height))
            .fold(Vec::<u16>::with_capacity((height * width * 6) as usize), // 6 indexed vertices per tile
                  |mut acc, tile_index|
                  { acc.extend_from_slice(&TextBlock::generate_tile_indices(tile_index as u32));
                    acc });

        let ibo =
            match glium::IndexBuffer::persistent(glium,
                                                 glium::index::PrimitiveType::TrianglesList,
                                                 &indices)
        {
            Ok(i)  => i,
            Err(e) => return Err(TextBlockErr::IBOCreation(e))
        };

        let tb = TextBlock{
            atlas:  atlas.dimensions.clone(),
            width:  width,
            height: height,
            block:  final_block,
            vbo:    vbo,
            ibo:    ibo,
            scanline_y: 0.0f32
        };

        tb.update();

        Ok(tb)
    }

    pub fn draw(&mut self,
                program: &glium::Program,
                target: &mut glium::Frame,
                atlas: &Atlas){
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

        let uniforms = uniform! {
            tex: &atlas.texture,
            matrix: *matrix.as_ref(),
            scanline_y: self.scanline_y
        };

        target.draw(&self.vbo, &self.ibo, program, &uniforms,
                    &Default::default()).expect("Failed to draw");


        self.scanline_y += 1.0f32 / 5.0f32 + 0.02f32; // this is just some random number tbh. Should appear like multiple bars crawling up the screen.
        if self.scanline_y > 1.2{   // if greater than the size of the screen and a little bit
            self.scanline_y -= 1.4; // set the scanline to just before the screen by a little bit (-0.4)
        }
    }
}
