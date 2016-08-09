#[macro_use]
extern crate glium;
extern crate image;
extern crate rustc_serialize;
extern crate nalgebra;

mod atlas;
mod textblock;

use atlas::Atlas;
use textblock::*;

use std::fs::File;
use std::io::Read;

fn main() {
    use glium::{DisplayBuild, Surface};
    use std::path::Path;
    let display = glium::glutin::WindowBuilder::new().build_glium().unwrap(); // XXX change to .expect()

    let vertex_shader_src = r#"
        #version 140

        in vec2   position;
        in vec2   texcoord;
        out vec2  v_tex_coord;
        out float v_position_y;

        uniform mat3 matrix;

        void main() {
            v_tex_coord = texcoord;
            gl_Position = vec4(matrix * vec3(position, 1.0), 1.0);
            v_position_y = gl_Position.y;
        }
    "#;

    let fragment_shader_src = r#"
        #version 140

        in vec2  v_tex_coord;
        in float v_position_y;
        out vec4 color;

        uniform sampler2D tex;
        uniform float     scanline_y;

        void main() {
            float scantensity = max(0,1.0 - (distance(-v_position_y, scanline_y*2 - 1.0) * 5.0));
            color = texture(tex, v_tex_coord) * max(0.5, scantensity);
            float increase = (scantensity*scantensity) * 0.05;
            color.b = color.b + increase;
            color.g = color.g + increase;
        }
    "#;

    let program = glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None).unwrap();

    let atl = match Atlas::new_from_file_blocking(&display, Path::new("atlas.json")){
        Ok(ok) => ok,
        Err(e) => return println!("Failed to load Atlas {:?}", e)
    };

    display
        .get_window()
        .unwrap()
        .set_inner_size(800,
                        600);

    let mut textvec: Vec<u8> = Vec::new();
    match File::open("screen.init"){
        Ok(mut file) => { file.read_to_end(&mut textvec);
                          textvec = textvec.iter().map(|&x| (-0x20i32 + x as i32) as u8).collect(); },
        Err(_) => { textvec = (0u32..(80u32*25u32)).map(|x| (0x10 + x % 10) as u8).collect(); }
    };


    let mut tb = match TextBlock::new(&display, &atl, 80, 25, Some(&textvec)){
        Ok(ok) => ok,
        Err(e) => return println!("Failed to create TextBlock {:?}", e)
    };

    loop {
        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);

        tb.draw(&program, &mut target, &atl);

        target.finish().unwrap();

        for ev in display.poll_events() {
            match ev {
                glium::glutin::Event::Closed => return,
                _ => ()
            }
        }
    }
}
