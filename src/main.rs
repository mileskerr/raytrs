extern crate png;

//https://www.desmos.com/calculator/i19ibmp3yt

use std::env;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::time::Instant;
use std::io::BufWriter;
use std::process::exit;
use std::error;
use std::collections::HashMap;

//use crate::png;


use std::thread;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;


mod scn;
mod space;

pub use space::*;

const HELP: &str = r#"
Usage: raytrs [OPTION]...

    -h, --help                      show this message
    -q, --quiet                     quiet mode, only print render time to stdout
    -s, --scene <filename.json>     set scene file. if no scene is provided, a
                                    very simple example will be rendered.
    -o, --output <filename.png>     set output file. defaults to render.png
    -t, --threads <# of threads>    set number of threads used. should be >= the
                                    number of logical cores in your system,
                                    defaults to 32
    -r, --resolution <WIDTHxHEIGHT> set image dimensions. defaults to 256x256
        --samples <# of samples>    if set to a nonzero value, will enable soft
                                    shadows using the set amount of random samples.
                                    warning: experimental, and basically useless.
                                    acceptable results are not possible without
                                    increasing render times by several orders of
                                    magnitude.
"#;
const GET_HELP: &str =
"\n- see \'raytrs --help\' for more info";
static mut QUIET: bool = false;

//multiply all brightnesses by this. 30 is pretty good
const EXPOSURE: f64 = 30.0;



fn main() {
    //all main does is check for errors. run() is where the fun begins
    match run() {
        Ok(()) => {
            exit(0);
        }
        Err(err) => {
            eprintln!("[raytrs] {}", err);
            exit(1);
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
    let mut samples: usize = 0;
    
    { //argument parsing
        let opts = [
            ("help", ClOpt::Flag{ action: &mut ( || {
                println!("{}",HELP);
                exit(0);
            })}),
            ("quiet", ClOpt::Flag{ action: &mut ( || {
                //trust me bro, im only gonna assign this variable once at the
                //very beginning you have nothing to worry about compiler
                unsafe { QUIET = true; };
            })}),
            ("output", ClOpt::Value{ action: &mut ( |filename: String| {
                output_file = filename;
                Ok(())
            })}),
            ("scene", ClOpt::Value{ action: &mut ( |filename: String| {
                scene_file = Some(filename);
                Ok(())
            })}),
            ("resolution", ClOpt::Value{ action: &mut ( |dimensions: String| {
                let invalid_res_error = &format!("invalid resolution {}", GET_HELP);
                let mut spl = dimensions.split('x');

                width = spl.next()
                    .ok_or(invalid_res_error)?
                    .parse().or(Err(invalid_res_error))?;
                height = spl.next()
                    .ok_or(invalid_res_error)?
                    .parse().or(Err(invalid_res_error))?;

                if width == 0 { return Err(format!("width cannot be zero")) };
                if height == 0 { return Err(format!("height cannot be zero")) };
                Ok(())
            })}),
            ("threads", ClOpt::Value{ action: &mut ( |t: String| {
                threads = t.parse().or(
                    Err(format!("invalid number of threads"))
                )?;
                if threads == 0 { return Err(format!("number of threads cannot be zero")); }
                Ok(())
            })}),
            ("samples", ClOpt::Value{ action: &mut ( |t: String| {
                samples = t.parse().or(
                    Err(format!("invalid number of samples"))
                )?;
                Ok(())
            })}),
        ];
        let names = [ //translation table for short names
            ("h","help"),
            ("q","quiet"),
            ("o","output"),
            ("s","scene"),
            ("r","resolution"),
            ("t","threads"),
        ];
        parse_args(&mut HashMap::from(opts),HashMap::from(names))?;
    }
  
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
    let pixels = scene.render(width,height,threads,samples)?; //render
    println!("done rendering in {} seconds", t0.elapsed().as_secs_f32());

    { //write file
        let file = File::create(&output_file)?;
        let ref mut w = BufWriter::new(file);

        let mut encoder = png::Encoder::new(w, width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;

        let mut data = Vec::new();
        for i in 0..width*height {
            data.push(pixels[i].r);
            data.push(pixels[i].g);
            data.push(pixels[i].b);
        }
        writer.write_image_data(&data)?;
        print_loud(format!("output written to \'{}\'\n", &output_file));
    }
    Ok(())
}

fn parse_args<'a>
(options: &mut HashMap<&'a str, ClOpt>,names: HashMap<&str,&'a str>) -> Result<(),String> {
    let mut do_option_action = | name: String, args: &mut std::env::Args | {
        match options.get_mut(&name[..]) {
            Some(ClOpt::Flag{action}) => { action(); }
            Some(ClOpt::Value{action}) => {
                //since next argument is inerpreted as the value of option, skip it.
                let arg_result = args.next().ok_or(
                    format!("must provide a value for \'{}\' {}",name, GET_HELP)
                )?;
                action(arg_result)?;
            }
            _=> {
                return Err(format!("invalid option \'{}\' {}",name, GET_HELP));
            }
        }
        Ok(())
    };


    let mut args = env::args();
    args.next(); //skip 0th argument

    //options are indexed by their long name, so first check if option provided is short by checking
    //number of dashes proceeding it, and if it is, loop through all characters in the argument
    //(for option chaining eg. ls -la), and translate each into the long name before setting them.
    while let Some(arg) = args.next() {
        if arg.find("--") == Some(0) {
            do_option_action(arg[2..].to_owned(), &mut args)?;
        } else if arg.find('-') == Some(0) {
            for i in 1..arg.len() {
                let short_name= &arg[i..i+1];
                let name = names.get(short_name).ok_or(
                    format!("invalid option \'{}\' {}",short_name, GET_HELP)
                )?;
                do_option_action(name.to_string(), &mut args)?;
            }
        }
        else {
            return Err(format!("invalid arguments {}", GET_HELP));
        }
    }
    return Ok(());
}

enum ClOpt<'a> { //types of command line options
    //either runs or doesn't, no data is passed into the closure
    Flag{ action: &'a mut dyn FnMut() }, 
    
    //a required next argument is passed into the closure
    Value{ action: &'a mut dyn FnMut(String) -> Result<(),String> }, 
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
    fn render(self, width: usize, height: usize, threads: usize, samples: usize) ->
    Result<Vec<Color>, String> {

        //higher is much better for large scenes
        const CHUNK_SIZE: usize = 256;

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
        if num_pixels % CHUNK_SIZE != 0 { //chunk at the end for leftover pixels. this won't be totally filled
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
                                        pixels[j] = Some(shade_reflective(ray,hit,&scene,3,samples));
                                    } else {
                                        pixels[j] = Some(shade_diffuse(hit,&scene.lights,&scene.objects,samples));
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

        //pixels is currently a vector of arrays of pixels,
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
}
fn shade_diffuse
(hit: RaycastHit, lights: &Vec<Light>, objects: &Vec<Box<dyn SceneObject + Send + Sync>>, samples: usize) -> Color {
    let mut lightness = 0.0;
    for light in lights {
        match light {
            Light::Point(point_light) => {
                //diffuse shading
                let light_vector = point_light.origin - hit.point;
                let light_distance = light_vector.magn();
                let light_dir = light_vector / light_distance;
                let mut l0 = (light_dir.dot(hit.normal)) * point_light.strength;
                if l0 < 0.0 { l0 = 0.0 } //clamp
                let mut new_light = ((l0 * l0) * EXPOSURE) / (light_distance * light_distance);
                //shadows
                for i in 0..objects.len() {
                    if samples == 0 {
                        let ray = Ray::new( hit.point, point_light.origin ) ;
                        let hit1 = objects[i].raycast( ray );
                        if hit1.is_some() {
                            new_light = 0.0;
                        }
                    }
                    else {
                        for _ in 0..samples {
                            let ray = Ray::new( hit.point, point_light.origin + (Vec3::random() * point_light.size)) ;
                            let hit1 = objects[i].raycast( ray );
                            if hit1.is_some() {
                                if hit1.unwrap().depth > 0.01 { //to prevent casting shadow on self
                                    new_light -= 1.0/(samples as f64);
                                }
                            }
                        }
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
fn shade_reflective
(ray: Ray, hit: RaycastHit, scene: &Scene, recurs_lim: u8, samples: usize) -> Color {
    let mut pixel = scene.world.color;
    let mut depth = f64::MAX;

    for i in 0..scene.objects.len() {
        let new_ray = Ray::new(hit.point, (ray.start - ray.end).unit().reflect(hit.normal) + hit.point);
        let refl_hit = scene.objects[i].raycast( new_ray );
        if refl_hit.is_some() && refl_hit.unwrap().depth < depth
        {
            let refl_hit = refl_hit.unwrap();
            depth = refl_hit.depth; 
            if recurs_lim > 0 && refl_hit.material.reflective {
                pixel = shade_reflective( new_ray, refl_hit, &scene, recurs_lim - 1, samples);
            } else {
                pixel = shade_diffuse( refl_hit, &scene.lights, &scene.objects, samples);
            }
        }
    }
    return pixel;
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
