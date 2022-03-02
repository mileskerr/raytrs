extern crate png;

use std::thread;
use std::env;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::time::Instant;
use std::io::BufWriter;
use std::error;
use std::fmt::Display;
use std::process::exit;

//use crate::png;


use std::sync::Arc;
use std::sync::Mutex;


mod scn;
mod space;

pub use space::*;

const EXPOSURE: f64 = 30.0;

//const NUM_PIXELS: usize = WIDTH * HEIGHT;
//const PIXELS_PER_THREAD: usize = NUM_PIXELS/THREADS;


fn main() {

    //basically all main does is handle errors. run() is where the fun begins
    match run() {
        Ok(()) => {
            exit(0);
        }
        Err(err) => {
            eprintln!("error: {}", err);
            reset_screen();
            exit(2);
        }
    }
}

fn run() -> Result<(),Box<dyn error::Error>> {
    
    let mut output_file = String::from("render.png");
    let mut scene_file: Option<String> = None;
    let mut width: usize = 256;
    let mut height: usize = 256;
    let mut threads: usize = 16;

    //parse args takes a vector of arguments, i have multiple types of arguments stored in an enum
    //all the types follow the basic structure of having a string name and a cloture to define what they do
    parse_args( vec![
        ClOpt::Flag {
            name: String::from("h"),
            action: &mut ( || {
                reset_screen();
                println!(r#"
usage:

[-h]                show this message
[-s <filename>]     set scene file (.json)
[-o <filename>]     set output file (.png, defaults to render.png)
[-d <widthxheight>] set image dimensions (defaults to 256x256)
[-t <# of threads>] set number of threads used (defaults to 16)

"#
                );
                exit(0);
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

    setup_screen();
  
    let t0 = Instant::now(); //render timer
    let scene = get_scene(scene_file)?;
    let pixels = scene.render(width,height,threads);
    write_file(pixels, width, height, output_file);
    
    reset_screen();
    println!("\n\ndone rendering in {} seconds\n", t0.elapsed().as_secs());
    Ok(())
}

fn get_scene(scene_file: Option<String>) -> Result<Scene, Box<dyn error::Error>> {

    let mut scene_contents = String::new();
    let mut scene_path = Path::new("./");
    match &scene_file {
        Some(file) => {
            scene_contents = fs::read_to_string(file)?;
            scene_path = Path::new(file);
        }
        _ => {
            scene_contents = scn::DEFAULT_JSON.to_string();
        }
    }
    let scene = scn::read_json(&scene_contents,scene_path)?;
    Ok(scene)
}

fn parse_args(mut options: Vec<ClOpt>) -> Result<(),ArgsError>{
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
                            //since next argument is value of current argument, skip it
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

enum ClOpt<'a> {
    Flag { name: String, action: &'a mut dyn FnMut() },
    Str { name: String, action: &'a mut dyn FnMut(String) -> Result<(),ArgsError> },
}

fn setup_screen() {
    print!(" {}{}{}{}",
        "\x1b[s",    //save cursor
        "\x1b[?47h", //save screen
        "\x1b[?25l", //hide cursor
        "\x1b[H"     //clear screen
    );
}
fn reset_screen() {
    print!("{}\n",
        //"\x1b[?47l", //restore screen
        //"\x1b[u",    //restore cursor
        "\x1b[?25h", //show cursor
    );
}

#[derive(Debug)]
struct ArgsError(String);
impl std::error::Error for ArgsError {}
impl Display for ArgsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}


fn write_file(pixels: Vec<Color>, width: usize, height: usize, filepath: String) {
    let file = File::create(filepath).unwrap();
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header().unwrap();

    let mut data = Vec::new();
    for i in 0..width * height {
        data.push(pixels[i].r);
        data.push(pixels[i].g);
        data.push(pixels[i].b);
    }

    writer.write_image_data(&data).unwrap();
}


pub struct Scene
{
    objects: Vec<Box<dyn SceneObject + Send + Sync>>,
    lights: Vec<Light>,
    camera: Camera,
    world: World,
}
impl Scene
{
    fn new(objects: Vec<Box<dyn SceneObject + Send + Sync>>, lights: Vec<Light>, camera: Camera, world: World) -> Scene
    {
        Scene { objects: objects, lights: lights, camera: camera, world: world }
    }
    fn render(self, width: usize, height: usize, threads: usize) -> Vec<Color> {
        println!("preparing... ");

        let num_pixels = width * height;
        let pixels_per_thread = num_pixels / threads;

        let scene = Arc::new(self);
        let dirs = Arc::new(scene.camera.dirs(width, height));
        let camera_origin = scene.camera.origin;
        let mut pixels: Vec<Arc<Mutex<Vec<Color>>>> = Vec::with_capacity(threads);
        let mut handles = Vec::with_capacity(threads);

        println!("rendering...");
        let cursor_offset = 5; //legit detecting cursor position is really hard. just hardcoding it

        for i in 0..threads {
            let formatted_i = if i > 9 { i.to_string() + ":" } else { i.to_string() + ": " };
            pixels.push(Arc::new(Mutex::new(Vec::with_capacity(pixels_per_thread))));
            let pixels = Arc::clone(&pixels[i]);
            let dirs = Arc::clone(&dirs);
            let scene = Arc::clone(&scene);

            let handle = thread::spawn(move || {
                //println!("thread {}:," 
                let mut pixels = pixels.lock().unwrap();
                let mut depths = Vec::new();
                for _ in 0..pixels_per_thread { //solid black background (really far away) first
                    pixels.push(scene.world.color);
                    depths.push(f64::MAX);
                }
                for k in 0..scene.objects.len() {
                    print!( //progress bar
                        "\x1b[{};0fthread {} {}",i+cursor_offset,formatted_i,progress_bar(k+1, scene.objects.len())
                    );
                    for j in 0..pixels_per_thread {
                        let ray = Ray::new(camera_origin, dirs[(i * pixels_per_thread) + j] + camera_origin);
                        let hit = scene.objects[k].raycast(ray);
                        if hit.is_some() {
                            let hit = hit.unwrap();
                            if hit.depth < depths[j] {
                                depths[j] = hit.depth;
                                if hit.material.reflective {
                                    pixels[j] = scene.shade_reflective(ray,hit);
                                } else {
                                    pixels[j] = scene.shade_diffuse(hit);
                                }
                            }
                        }
                    }
                }
            });
            handles.push(handle);
        }
        for handle in handles {
            handle.join().unwrap();
        }

        println!("\x1b[{};0fcleaning up...   ",cursor_offset+threads+1);

        let mut output = Vec::new(); //merge all the thread vectors into a single vector
        for thread in pixels {
            for pixel in thread.lock().unwrap().clone() {
                output.push(pixel);
            }
        }

        return output;
       
    }
    fn shade_diffuse(&self, hit: RaycastHit) -> Color
    {
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
fn progress_bar(value: usize,max: usize) -> String {
    const LEN: usize  = 32;
    const START: char = '[';
    const END: char   = ']';
    const FILL: char  = '#';
    const DIV: char   = '#';
    const EMPT: char  = '-';
    const DONE: &str  = "[              DONE              ]";

    if value >= max
    { return String::from(DONE); }

    let amount = if max == 0 { 0 } else { (value * LEN)/max };
    let empty = LEN - amount;

    let mut bar = String::with_capacity(LEN + 2);

    let length = if amount > 0 { amount - 1 } else { 0 };
    bar.push(START);
    for _ in 0..length {
        bar.push(FILL);
    }
    if amount > 0 { bar.push(DIV); }
    for _ in 0..empty {
        bar.push(EMPT);
    }
    bar.push(END);
    
    return bar;
}

impl Camera
{
    fn new( origin: Vec3, direction: Vec3, length: f64) -> Camera
    {
        Camera { origin: origin, direction: direction, length: length }
    }
    /*fn new(origin: Vec3, upper_left: Vec3, upper_right: Vec3, lower_left: Vec3, lower_right: Vec3) -> Camera
    { Camera
        { origin: origin, upper_left: upper_left, upper_right: upper_right,
        lower_left: lower_left, lower_right: lower_right, }
    }*/
    fn dirs(&self, width: usize, height: usize) -> Vec<Vec3>
    {
        //fix image getting shrunk vertically as camera direction changes
        println!("generating view rays...   ");

        let z_unit = self.direction.unit();
        let x_unit = Vec3::new(0.0,1.0,0.0).cross(z_unit).unit();
        //let y_unit = z_unit.cross(x_unit).unit();
        let y_unit = Vec3::new(0.0,1.0,0.0);

        let view_matrix = Matrix3::new(x_unit,y_unit,z_unit);

        let aspect = (width as f64) / (height as f64);
        let half = aspect/2.0;


        let upper_left  = Vec3::new(-half, 0.5,self.length);
        let upper_right = Vec3::new(half,  0.5,self.length);
        let lower_right = Vec3::new(half, -0.5,self.length);

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
        
        if a > -EPSILON && a < EPSILON
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
impl SceneObject for Floor
{
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



trait SceneObject
{ fn raycast(&self, ray: Ray) -> Option<RaycastHit>; }
