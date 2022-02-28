extern crate png;

use std::thread;
use std::env;
use std::fs::File;
use std::time::Instant;
use std::io::BufWriter;

//use crate::png;


use std::sync::Arc;
use std::sync::Mutex;


mod scn;
mod space;

pub use space::*;

const WIDTH:  usize = 256;
const HEIGHT: usize = 256;
const THREADS: usize = 16;

const NUM_PIXELS: usize = WIDTH * HEIGHT;
const PIXELS_PER_THREAD: usize = NUM_PIXELS/THREADS;


fn main() {
    let path = env::args()
        .nth(1)
        .expect("Expected a filename to output to.");


    print!("{}{}{}{}",
        "\x1b[?47h", //save screen
        "\x1b[s",    //save cursor
        "\x1b[?25l", //hide cursor
        "\x1b[H"     //clear screen
    ); 

    let scene: Scene = scn::generate_default();    
    let t0 = Instant::now();
    let pixels = scene.render();

    write_file(pixels, path);
    
    print!("{}{}{}{}{}{}",
        "\x1b[?47l", //restore screen
        "\x1b[u",    //restore cursor
        "\x1b[?25h", //show cursor
        "\ndone rendering in ", t0.elapsed().as_secs(), " seconds\n"
    );

}

fn write_file(pixels: Vec<Color>, filepath: String)
{
    let file = File::create(filepath).unwrap();
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, WIDTH as u32, HEIGHT as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header().unwrap();

    let mut data = [0;WIDTH*HEIGHT*3];
    for i in 0..NUM_PIXELS {
        let j = i * 3;
        data[j  ] = pixels[i].r;
        data[j+1] = pixels[i].g;
        data[j+2] = pixels[i].b;
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
    fn render(self) -> Vec<Color> {
        println!("preparing... ");

        let scene = Arc::new(self);
        let dirs = Arc::new(scene.camera.dirs());
        let camera_origin = scene.camera.origin;
        let mut pixels: Vec<Arc<Mutex<Vec<Color>>>> = Vec::with_capacity(THREADS);
        let mut handles = Vec::with_capacity(THREADS);

        println!("rendering...");
        let cursor_offset = 5; //legit detecting cursor position is really hard. just hardcoding it

        for i in 0..THREADS {
            let formatted_i = if i > 9 { i.to_string() + ":" } else { i.to_string() + ": " };
            pixels.push(Arc::new(Mutex::new(Vec::with_capacity(PIXELS_PER_THREAD))));
            let pixels = Arc::clone(&pixels[i]);
            let dirs = Arc::clone(&dirs);
            let scene = Arc::clone(&scene);

            let handle = thread::spawn(move || {
                //println!("thread {}:," 
                let mut pixels = pixels.lock().unwrap();
                let mut depths = Vec::new();
                for _ in 0..PIXELS_PER_THREAD { //solid black background (really far away) first
                    pixels.push(Color::new(0,0,0,255));
                    depths.push(f64::MAX);
                }
                for k in 0..scene.objects.len() {
                    print!( //progress bar
                        "\x1b[{};0fthread {} {}",i+cursor_offset,formatted_i,progress_bar(k+1, scene.objects.len())
                    );
                    for j in 0..PIXELS_PER_THREAD {
                        let ray = Ray::new(camera_origin, dirs[(i * PIXELS_PER_THREAD) + j] + camera_origin);
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

        println!("\x1b[{};0fcleaning up...   ",cursor_offset+THREADS+1);

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
                    let light_dir = light_vector / light_vector.magn();
                    let mut l0 = (light_dir.dot(hit.normal)+0.1) * point_light.strength;
                    if l0 < 0.0 //clamp because we dont want negative values messing things up
                    { l0 = 0.0 }
                    let mut new_light = l0 * l0;
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
    const LEN: usize = 32;
    const START: char = '[';
    const END: char = ']';
    const FILL: char = '#';
    const EMPT: char = '-';
    const DONE: &str = "[              DONE              ]";

    if value >= max
    { return String::from(DONE); }

    let amount = if max == 0 { 0 } else { (value * LEN)/max };
    let empty = LEN - amount;

    let mut bar = String::with_capacity(LEN + 2);

    bar.push(START);
    for _ in 0..amount {
        bar.push(FILL);
    }
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
        let z_unit = direction.unit();
        let x_unit = Vec3::new(0.0,1.0,0.0).cross(z_unit).unit();
        let y_unit = z_unit.cross(x_unit);

        let view_matrix = Matrix3::new(x_unit,y_unit,z_unit);

        let aspect = (HEIGHT as f64) / (WIDTH as f64);
        let half = aspect/2.0;

        let upper_left  = view_matrix * Vec3::new(-half, 0.5,length);
        let upper_right = view_matrix * Vec3::new(half,  0.5,length);
        let lower_left  = view_matrix * Vec3::new(-half,-0.5,length);
        let lower_right = view_matrix * Vec3::new(half, -0.5,length);

        Camera { origin: origin, upper_left: upper_left, upper_right: upper_right,
        lower_left: lower_left, lower_right: lower_right, }
    }
    /*fn new(origin: Vec3, upper_left: Vec3, upper_right: Vec3, lower_left: Vec3, lower_right: Vec3) -> Camera
    { Camera
        { origin: origin, upper_left: upper_left, upper_right: upper_right,
        lower_left: lower_left, lower_right: lower_right, }
    }*/
    fn dirs(&self) -> Vec<Vec3>
    {
        println!("generating view rays...   ");
        let mut dir = self.upper_left;

        let dx = (self.upper_right.x - self.upper_left.x)/(WIDTH as f64);
        let dy = (self.upper_right.y - self.lower_right.y)/(HEIGHT as f64);

        let mut dirs = Vec::with_capacity(NUM_PIXELS);

        for _y in 0..HEIGHT {
            dir.x = self.upper_left.x;
            for _x in 0..WIDTH {
                dirs.push(dir);
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
