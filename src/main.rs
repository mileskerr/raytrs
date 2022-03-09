extern crate png;

//https://www.desmos.com/calculator/i19ibmp3yt

use std::env;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::time::Instant;
use std::io::BufWriter;
use std::fmt::Display;
use std::process::exit;
use std::error;

//use crate::png;


use std::thread;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;


mod scn;
mod space;

pub use space::*;

const EXPOSURE: f64 = 30.0;

static mut QUIET: bool = false;


fn main() {
    //all main does is check for errors. run() is where the fun begins
    match run() {
        Ok(()) => {
            exit(0);
        }
        Err(err) => {
            eprintln!("error: {}", err);
            exit(2);
        }
    }
}

fn run() -> Result<(),Box<dyn error::Error>> {
   
    //defaults
    let mut output_file = String::from("render.png");
    let mut scene_file: Option<String> = None;
    let mut width: usize = 256;
    let mut height: usize = 256;
    let mut threads: usize = 32;

    parse_args( vec![
        ClOpt::Flag {
            name: String::from("h"),
            action: &mut ( || {
                println!(r#"
usage:
[-h]                show this message
[-q]                quiet mode, only write render time to stdout
[-s <filename>]     set scene file (.json)
[-o <filename>]     set output file (.png, defaults to render.png)
[-d <WIDTHxHEIGHT>] set image dimensions (defaults to 256x256)
[-t <# of threads>] set number of threads used (should be >= number of logical cores in your system, defaults to 32)
"#
                );
                exit(0);
            }),
        },
        ClOpt::Flag {
            name: String::from("q"),
            action: &mut ( || {
                //trust me bro, im only gonna assign this variable once at the
                //very beginning you have nothing to worry about compiler
                unsafe { QUIET = true; };
            }),
        },
        ClOpt::Str {
            name: String::from("o"),
            action: &mut ( |filename| {
                output_file = filename;
                Ok(())
            }),
        },
        ClOpt::Str {
            name: String::from("s"),
            action: &mut ( |filename| {
                scene_file = Some(filename);
                Ok(())
            }),
        },
        ClOpt::Str {
            name: String::from("d"),
            action: &mut ( |dimensions| {
                let mut spl = dimensions.split('x');
                width = spl.next().ok_or(
                        ArgsError("dimensions should be in format: <WIDTHxHEIGHT>".to_string())
                    )?.parse().or(
                        Err(ArgsError("invalid width".to_string()))
                    )?;
                height = spl.next().ok_or(
                        ArgsError("dimensions should be in format: <WIDTHxHEIGHT>".to_string())
                    )?.parse().or(
                        Err(ArgsError("invalid height".to_string()))
                    )?;
                if width == 0 { return Err(ArgsError("width cannot be zero".to_string())) };
                if height == 0 { return Err(ArgsError("height cannot be zero".to_string())) };
                Ok(())
            }),
        },
        ClOpt::Str {
            name: String::from("t"),
            action: &mut ( |t| {
                threads = t.parse().or(
                    Err(ArgsError("invalid number of threads".to_string()))
                )?;
                if threads == 0 { return Err(ArgsError("number of threads cannot be zero".to_string())); }
                Ok(())
            }),
        },
    ])?;

  
    print_loud(format!("loading scene...\n"));

    let scene = {
        let mut scene_path = Path::new("./");
        let scene_contents = match &scene_file {
            Some(file) => {
                scene_path = Path::new(file);
                fs::read_to_string(file)?
            }
            None => { scn::DEFAULT_JSON.to_string() }
        };
        scn::read_json(&scene_contents,scene_path)?
    };

    let t0 = Instant::now(); //render timer
    let pixels = scene.render(width,height,threads)?; //render
    println!("done rendering in {} seconds", t0.elapsed().as_secs_f32());

    { //write file
        let file = File::create(&output_file).unwrap();
        let ref mut w = BufWriter::new(file);

        let mut encoder = png::Encoder::new(w, width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header().unwrap();

        let mut data = Vec::new();
        for i in 0..width*height {
            data.push(pixels[i].r);
            data.push(pixels[i].g);
            data.push(pixels[i].b);
        }
        writer.write_image_data(&data).unwrap();
        print_loud(format!("output written to \"{}\"\n", &output_file));
    }
    
    Ok(())
}


fn parse_args(mut options: Vec<ClOpt>) -> Result<(),ArgsError>{
    //checks for arguments and executes the closure provided for each one,
    //with read data passed into the closure depending on the command line option type
    let mut args = env::args();
    args.next(); //skip 0th argument
    while let Some(arg) = args.next() {
        if arg.find('-') == Some(0) {
            let arg = arg[1..2].to_string();
            for opt in &mut options {
                match opt {
                    ClOpt::Flag{ name, action } => {
                        if arg == *name {
                            action();
                            break;
                        }
                    }
                    ClOpt::Str{ name, action } => {
                        if arg == *name {
                            //since next argument is inerpreted as the value of current argument, skip it
                            let arg_result = args.next().ok_or(
                                ArgsError("no value specified for -".to_string() + 
                                &name.to_string())
                            )?;
                            action(arg_result)?;
                        }
                    }
                }
            }
        }
        else {
            return Err(ArgsError("invalid arguments supplied. see -h for usage".to_string()));
        }
    }
    return Ok(());
}

enum ClOpt<'a> { //types of command line options
    //either runs or doesn't, no data is passed into the closure
    Flag { name: String, action: &'a mut dyn FnMut() }, 
    
    //a required next argument is passed into the closure
    Str { name: String, action: &'a mut dyn FnMut(String) -> Result<(),ArgsError> }, 
}

#[derive(Debug)]
struct ArgsError(String);
impl std::error::Error for ArgsError {}
impl Display for ArgsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn print_loud(content: String) {
    if unsafe { !QUIET } {
        print!("{}",content);
    }
}


pub struct Scene
{
    objects: Vec<Box<dyn SceneObject + Send + Sync>>,
    lights: Vec<Light>,
    camera: Camera,
    world: World,
}
impl Scene {
    fn new(
        objects: Vec<Box<dyn SceneObject + Send + Sync>>, lights: Vec<Light>, camera: Camera, world: World
    ) -> Scene {
        Scene { objects: objects, lights: lights, camera: camera, world: world }
    }
    fn render(self, width: usize, height: usize, threads: usize) -> Result<Vec<Color>, String> {

        //higher value means threads spend more time sitting around at the end of the render,
        //lower value means more overhead spawning and closing threads. 
        //higher is probably better for heavy scenes.
        const CHUNK_SIZE: usize = 1024;

        let num_pixels = width * height;
        let chunks = num_pixels/CHUNK_SIZE;
        
        let scene = Arc::new(self);
        let dirs = Arc::new(scene.camera.dirs(width, height));
        let camera_origin = scene.camera.origin;
        let mut pixels: Vec<Arc<Mutex<[Option<Color>;CHUNK_SIZE]>>> = Vec::with_capacity(chunks);
        
        let mut chunk_status: Vec<u8> = Vec::new(); //0=unrendered, 1=in progress, 2=done
        for _ in 0..chunks {
            pixels.push(Arc::new(Mutex::new([None;CHUNK_SIZE])));
            chunk_status.push(0); 
        }
        if num_pixels % CHUNK_SIZE != 0 {
            pixels.push(Arc::new(Mutex::new([None;CHUNK_SIZE])));
        }
        
        
       
        //channel threads use to communicate that they finished their chunk.
        //main thread will start a new thread occupied with an unrendered chunk
        //upon recieving the message.
        let (tx, rx) = mpsc::channel();
        let mut handles = Vec::with_capacity(threads);
       
        //threads are started initially by sending the message that all threads
        //have finished doing nothing, and need to be given work.
        for _ in 0..threads { tx.send(None).unwrap(); }


        loop {
            let done = rx.recv().unwrap(); //loop waits to recieve message that a thread is done
          
            let new_chunk = { //get next chunk to render
                if done.is_some() {
                    chunk_status[done.unwrap()] = 2;
                }
                let mut new_chunk: Option<usize> = None;
                for i in 0..chunks {
                    if chunk_status[i] == 0 {
                        chunk_status[i] = 1;
                        new_chunk = Some(i);
                        break;
                    }
                }
                new_chunk
            };

            if unsafe {!QUIET} { //progress indicator
                let aspect = (height as f32) / (width as f32);
                let line_length: usize = ((chunks as f32) / aspect).sqrt() as usize;
                if new_chunk.is_none() || new_chunk.unwrap() > 0 {
                    print!("\x1b[{}A\n",chunks/line_length+2);
                }
                let mut done_chunks = 0;
                for i in 0..chunks {
                    if chunk_status[i] == 2 { done_chunks +=1; }
                }
                print!("rendering on {} threads... {}/{}\n",threads,done_chunks,chunks);
                for i in 0..chunks {
                    match chunk_status[i] {
                        0 => { print!("░░"); }
                        1 => { print!("▒▒"); }
                        _ => { print!("▓▓"); }
                    }
                    if (i+1) % line_length == 0 && i+1 < line_length * (chunks/line_length) {
                        print!("\n");
                    }
                }
                print!("\n");
            }

            if new_chunk.is_some() { //render the chunk in a new thread, meanwhile restart the loop
                let chunk_index = new_chunk.unwrap();
                let dirs = Arc::clone(&dirs);
                let scene = Arc::clone(&scene);
                let pixels = Arc::clone(&pixels[chunk_index]);
                let tx = tx.clone();
                let handle = thread::spawn(move || { //actual rendering code here:
                    let mut pixels = pixels.lock().unwrap();
                    let mut depths = Vec::new();
                    for i in 0..CHUNK_SIZE { //fill background (really far away) first
                        pixels[i] = Some(scene.world.color);
                        depths.push(f64::MAX);
                    }
                    for k in 0..scene.objects.len() {
                        for j in 0..CHUNK_SIZE {
                            let dir = dirs[(chunk_index * CHUNK_SIZE) + j];
                            let ray = Ray::new(camera_origin, dir + camera_origin);
                            let hit = scene.objects[k].raycast(ray);
                            if hit.is_some() {
                                let hit = hit.unwrap();
                                if hit.depth < depths[j] {
                                    depths[j] = hit.depth;
                                    if hit.material.reflective {
                                        pixels[j] = Some(scene.shade_reflective(ray,hit));
                                    } else {
                                        pixels[j] = Some(scene.shade_diffuse(hit));
                                    }
                                }
                            }
                        }
                    }
                    tx.send(Some(chunk_index)).unwrap();
                });
                handles.push(handle);
            }
            //loop keeps running even after there are no chunks to assign,
            //but it has to stop when they are all finished rendering
            else if !(chunk_status.contains(&1)) {
                break;
            }
        }
        for handle in handles {
            handle.join().unwrap();
        }

        //pixels is currently a vector of vector of pixels,
        //merge it into a single vector of pixels:
        let mut output: Vec<Color> = Vec::new(); 
        for thread in pixels {
            for pixel in thread.lock().unwrap().clone() {
                if pixel.is_some() {
                    output.push(pixel.unwrap());
                }
                else {
                    output.push(Color::new(0,0,0,255));
                }
            }
        }
        Ok(output)
    }
    fn shade_diffuse(&self, hit: RaycastHit) -> Color {
        let mut lightness = 0.0;
        for light in &self.lights {
            match light {
                Light::Point(point_light) => {
                    //diffuse shading
                    let light_vector = point_light.origin - hit.point;
                    let light_distance = light_vector.magn();
                    let light_dir = light_vector / light_distance;
                    let mut l0 = (light_dir.dot(hit.normal)) * point_light.strength;
                    if l0 < 0.0 //clamp because we dont want negative values messing things up
                    { l0 = 0.0 }
                    let mut new_light = ((l0 * l0) * EXPOSURE) / (light_distance * light_distance);
                    //shadows
                    for i in 0..self.objects.len() {
                        let ray = Ray::new( hit.point, point_light.origin);
                        let hit1 = &self.objects[i].raycast( ray );
                        if hit1.is_some() {
                            new_light = 0.0;
                        }
                    }
                    lightness = lightness + new_light;
                }
                _ => {}
            }
        }
        let pixel = hit.material.color * lightness;
        return pixel;
    }
    fn shade_reflective(&self, ray: Ray, hit: RaycastHit) -> Color {
        let mut pixel = self.world.color;

        for object_index_1 in 0..self.objects.len() {
            let new_ray = Ray::new(hit.point, (ray.start - ray.end).unit().reflect(hit.normal) + hit.point);
            let hit1 = &self.objects[object_index_1].raycast( new_ray );
            if hit1.is_some()
            {
                pixel = self.shade_diffuse( hit1.unwrap() );
            }
        }
        return pixel;
    }
}

impl Camera {
    fn new( origin: Vec3, direction: Vec3, length: f64) -> Camera {
        Camera { origin: origin, direction: direction, length: length }
    }
    fn dirs(&self, width: usize, height: usize) -> Vec<Vec3> {
        print_loud(format!("generating view rays...\n"));

        //first do matrix math to transform the easy-to-understand
        //camera properties into something that's actually useful:
        let z_unit = self.direction.unit();
        let x_unit = Vec3::new(0.0,1.0,0.0).cross(z_unit).unit();
        let y_unit = Vec3::new(0.0,1.0,0.0);

        let view_matrix = Matrix3::new(x_unit,y_unit,z_unit);

        let aspect = (width as f64) / (height as f64);
        let half = aspect/2.0;

        let upper_left  = Vec3::new(-half, 0.5,self.length);
        let upper_right = Vec3::new(half,  0.5,self.length);
        let lower_right = Vec3::new(half, -0.5,self.length);



        //iterate through pixels and calculate their direction:
        let mut dir = upper_left;

        let dx = (upper_right.x - upper_left.x)/(width as f64);
        let dy = (upper_right.y - lower_right.y)/(height as f64);

        let num_pixels = width * height;
        let mut dirs = Vec::with_capacity(num_pixels);

        for _y in 0..height {
            dir.x = upper_left.x;
            for _x in 0..width {
                dirs.push(view_matrix * dir);
                dir.x += dx;
            }
            dir.y -= dy;
        }
        return dirs;

    }
}


impl SceneObject for Tri {
    fn raycast(&self, ray: Ray) -> Option<RaycastHit> {
        //Moller-Trumbore algorithm:
        const EPSILON: f64 = 0.000001;
        
        let dir = (ray.end - ray.start).unit();
        
        let edge0 = self.verts.1 - self.verts.0;
        let edge1 = self.verts.2 - self.verts.0;
        
        let h = dir.cross(edge1);
        let a = edge0.dot(h);
        
        if a < EPSILON
        { return None; }
        
        let f = 1.0/a;
        let s = ray.start - self.verts.0;
        let u = f * (s.dot(h));
        
        if u < 0.0 || u > 1.0
        { return None; }

        
        let q = s.cross(edge0);
        let v = f * (dir.dot(q));
        
        if v < 0.0 || (u + v) > 1.0
        { return None; }
        
        let t = f * (edge1.dot(q));
        
        if t > EPSILON {
            let point = ray.start + (dir * t);

            let normal = (self.vx_normals.1 * u) + (self.vx_normals.2 * v) + (self.vx_normals.0 * (1.0 - u - v)); 

            return Some(RaycastHit::new(point, normal, t, self.material ));
        }
        else
        { return None; }
    }
}

impl SceneObject for Sphere {
    fn raycast(&self, ray: Ray) -> Option<RaycastHit> {
        let delta = (ray.end - ray.start).unit();
       
        let a = delta.dot(delta);
        let b = (delta * 2.0).dot(ray.start - self.center);
        let c = self.center.dot(self.center) + 
                ray.start.dot(ray.start) - 
                2.0 * self.center.dot(ray.start) - 
                self.radius * self.radius;

        let dsc = b * b - (4.0 * a * c);

        let mut hit: Option<RaycastHit> = None;

        if dsc >= 0.0 {
            let t = (- b - dsc.sqrt()) / (2.0 * a);
            let point = ray.start + (delta * t);
            if (point - ray.start).dot(delta) > 0.0 { //check that sphere is not behind ray
                let normal = (point - self.center)/self.radius;
                hit = Some(RaycastHit::new(point, normal, t, self.material));
            }
        }
        return hit;
    }
}
impl SceneObject for Floor {
    fn raycast(&self, ray: Ray) -> Option<RaycastHit> {
        let dir = (ray.end - ray.start).unit();
        if dir.y >= 0.0 {
            return None;
        }
        else {
            let t = (self.y - ray.start.y) / dir.y;
            let point = dir * t + ray.start;

            return Some(RaycastHit::new(point, Vec3::new(0.0,1.0,0.0), t, self.material));
        }
    }
}



trait SceneObject {
    //check intersection of self and a given ray
    fn raycast(&self, ray: Ray) -> Option<RaycastHit>;
}
