#[macro_use]
extern crate glium;
extern crate image;
extern crate rustc_serialize;
extern crate nalgebra;

mod atlas;
mod tileblock;

use atlas::Atlas;
use tileblock::*;

fn main() {
    use glium::{DisplayBuild, Surface};
    use std::path::Path;
    let display = glium::glutin::WindowBuilder::new().build_glium().unwrap(); // XXX change to .expect()

    let vertex_shader_src = r#"
        #version 140

        in vec2 position;
        in vec2 texcoord;
        out vec2 v_tex_coord;

        uniform mat3 matrix;

        void main() {
            v_tex_coord = texcoord;
            gl_Position = vec4(matrix * vec3(position, 1.0), 1.0);
        }
    "#;

    let fragment_shader_src = r#"
        #version 140

        in vec2 v_tex_coord;
        out vec4 color;

        uniform sampler2D tex;

        void main() {
            color = texture(tex, v_tex_coord);
        }
    "#;

    let program = glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None).unwrap();

    let atl = match Atlas::new_from_file_blocking(&display, Path::new("atlas.json")){
        Ok(ok) => ok,
        Err(e) => return println!("Failed to load Atlas {:?}", e)
    };

    let textvec: Vec<u8> = "Hello world".chars().map(|x| (x as u8 - 0x20) as u8).collect();

    let mut tb = match TileBlock::new(&display, &atl, 11, 1, Some(&textvec)){
        Ok(ok) => ok,
        Err(e) => return println!("Failed to create TileBlock {:?}", e)
    };

    let mut t = 0.0f32;

    loop {
        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);

        tb.draw(&program, &mut target, &atl, [0.0, 0.0]);

        target.finish().unwrap();

        for ev in display.poll_events() {
            match ev {
                glium::glutin::Event::Closed => return,
                _ => ()
            }
        }

        t = t + 0.01f32;
    }
}
