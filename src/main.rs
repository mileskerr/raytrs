use png;
use std::thread;
use std::env;
use std::fs::File;
use std::time::Instant;
use std::io::BufWriter;

use std::sync::Arc;
use std::sync::Mutex;

mod scn;
mod space;

pub use space::*;

const WIDTH:  usize = 300;
const HEIGHT: usize = 300;
const THREADS: usize = 8;

const RAYS_PER_THREAD: usize = (WIDTH*HEIGHT)/THREADS;

fn main()
{
    let path = env::args()
        .nth(1)
        .expect("Expected a filename to output to.");

    let scene: Scene = scn::generate_default();    
    let pixels = scene.render();

    write_file(pixels, path);

}

fn write_file(pixels: [Color;WIDTH*HEIGHT], filepath: String)
{
    let file = File::create(filepath).unwrap();
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, WIDTH as u32, HEIGHT as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header().unwrap();

    let mut data = [0;WIDTH*HEIGHT*3];
    for i in 0..WIDTH*HEIGHT
    {
        let j = i * 3;
        data[j  ] = pixels[i].r;
        data[j+1] = pixels[i].g;
        data[j+2] = pixels[i].b;
    }

    writer.write_image_data(&data).unwrap();
}


pub struct Scene
{
    objects: Vec<Arc<dyn SceneObject + Send + Sync>>,
    lights: Vec<Light>,
    camera: Camera,
    world: World,
}
impl Scene
{
    fn new(objects: Vec<Arc<dyn SceneObject + Send + Sync>>, lights: Vec<Light>, camera: Camera, world: World) -> Scene
    {
        Scene { objects: objects, lights: lights, camera: camera, world: world }
    }
    fn render(self) -> [Color;WIDTH*HEIGHT]
    {
        let t0 = Instant::now();

        let self_arc = Arc::new(self);
        let self1 = Arc::clone(&self_arc);
        let rays = self1.camera.rays();

        let mut comp_pixels: Arc<Mutex<[Color;WIDTH*HEIGHT]>> = Arc::new(Mutex::new([self1.world.color;WIDTH*HEIGHT]));
        let mut comp_depth_buffer: Arc<Mutex<[f64;WIDTH*HEIGHT]>> = Arc::new(Mutex::new([1000.0;WIDTH*HEIGHT]));
       
        println!("rendering...   ");
        let mut object_index = 0;
        for object in &self1.objects
        {
            print!("\x1b[s{}/{}\x1b[u",object_index,&self1.objects.len());
            object_index += 1;

            let mut handles = Vec::new();
            
            for i in 0..THREADS
            {
                let rays_vec = Arc::clone(&rays[i]);
                let obj_clone = Arc::clone(&object);
                let self_clone = Arc::clone(&self_arc);
                let comp_pixels_clone = Arc::clone(&comp_pixels);
                let comp_db_clone = Arc::clone(&comp_depth_buffer);


                let handle = thread::spawn(move ||
                {
                    //let mut pixels: [Color;RAYS_PER_THREAD] = [self_clone.world.color;RAYS_PER_THREAD];
                    //let mut depth_buffer: [f64;RAYS_PER_THREAD] = [1000.0;RAYS_PER_THREAD];
                    for j in 0..RAYS_PER_THREAD
                    {
                        let hit = obj_clone.raycast(rays_vec[j]);
                        if hit.is_some()
                        {
                            let db = &mut comp_db_clone.lock().unwrap()[j + (i * RAYS_PER_THREAD)];
                            if hit.unwrap().depth < *db
                            {
                                *db = hit.unwrap().depth;
                                if hit.unwrap().material.reflective
                                {
                                    comp_pixels_clone.lock().unwrap()[j + (i * RAYS_PER_THREAD)] =
                                        self_clone.shade_reflective(rays_vec[j],hit.unwrap());
                                } else
                                {
                                    comp_pixels_clone.lock().unwrap()[j + (i * RAYS_PER_THREAD)] =
                                        self_clone.shade_diffuse(hit.unwrap());
                                }
                            }
                        }
                    }
                });
                handles.push(handle);
            }

            for handle in handles
            { handle.join().unwrap(); }
        }
        println!("finished rendering in {} secs", t0.elapsed().as_secs());
       
        return *comp_pixels.lock().unwrap();
    }
    fn shade_diffuse(&self, hit: RaycastHit) -> Color
    {
        let mut lightness = 0.0;
        for light in &self.lights
        {
            match light
            {
                Light::Point(point_light) =>
                {
                    //diffuse shading
                    let light_vector = point_light.origin - hit.point;
                    let light_dir = light_vector / light_vector.magn();
                    let mut l0 = light_dir.dot(hit.normal) * point_light.strength;
                    if l0 < 0.0 //clamp because we dont want negative values messing things up
                    { l0 = 0.0 }
                    let mut new_light = l0 * l0;
                    //shadows
                    for i in 0..self.objects.len()
                    {
                        let ray = Ray::new( hit.point, point_light.origin);
                        let hit1 = &self.objects[i].raycast( ray );
                        if hit1.is_some()
                        {
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
    fn shade_reflective(&self, ray: Ray, hit: RaycastHit) -> Color
    {
        let mut pixel = self.world.color;

        for object_index_1 in 0..self.objects.len()
        {
            let new_ray = Ray::new(hit.point, (ray.start - ray.end).reflect(hit.normal) + hit.point);
            let hit1 = &self.objects[object_index_1].raycast( new_ray );
            if hit1.is_some()
            {
                pixel = self.shade_diffuse( hit1.unwrap() );
            }
        }
        return pixel;
    }
}


impl Camera
{
    fn new(origin: Vec3, upper_left: Vec3, upper_right: Vec3, lower_left: Vec3, lower_right: Vec3) -> Camera
    { Camera
        { origin: origin, upper_left: upper_left, upper_right: upper_right,
        lower_left: lower_left, lower_right: lower_right, far_clip: 20.0, }
    }
    fn rays(&self) -> Vec<Arc<Vec<Ray>>>
    {
        let mut non_arc = Vec::new();
        let mut dir = self.upper_left;

        let num_pixels = WIDTH * HEIGHT;

        for i in 0..THREADS
        { 
            non_arc.push(Vec::new());
        }

        let dx = (self.upper_right.x - self.upper_left.x)/(WIDTH as f64);
        let dy = (self.upper_right.y - self.lower_right.y)/(HEIGHT as f64);

        let mut thread_index = 0;

        for y in 0..HEIGHT
        {
            dir.x = self.upper_left.x;
            for x in 0..WIDTH
            {
                if (WIDTH * y + x) >= RAYS_PER_THREAD * thread_index + RAYS_PER_THREAD
                { thread_index += 1; }
                let ray_end = dir.unit() + self.origin;
                non_arc[thread_index].push(Ray::new(self.origin, ray_end));
                dir.x += dx;
            }
            dir.y -= dy;
        }
        let mut output = Vec::new();
        for i in 0..THREADS
        {
            output.push(Arc::new(non_arc[i].clone()));
        }
        return output;
    }
}


impl SceneObject for Tri
{
    fn raycast(&self, ray: Ray) -> Option<RaycastHit>
    {
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
        
        if t > EPSILON
        {
            let point = ray.start + (dir * t);

            let normal = (self.vx_normals.1 * u) + (self.vx_normals.2 * v) + (self.vx_normals.0 * (1.0 - u - v)); 

            return Some(RaycastHit::new(point, normal, t, self.material ));
        }
        else
        { return None; }
    }
}

impl SceneObject for Sphere
{
    fn raycast(&self, ray: Ray) -> Option<RaycastHit>
    {
        let delta = (ray.end - ray.start).unit();
       
        let a = delta.dot(delta);
        let b = (delta * 2.0).dot(ray.start - self.center);
        let c = self.center.dot(self.center) + 
                ray.start.dot(ray.start) - 
                2.0 * self.center.dot(ray.start) - 
                self.radius * self.radius;

        let dsc = b * b - (4.0 * a * c);

        let mut hit: Option<RaycastHit> = None;

        if dsc >= 0.0
        {
            let t = (- b - dsc.sqrt()) / (2.0 * a);
            let point = ray.start + (delta * t);
            if (point - ray.start).dot(delta) > 0.0 //check that sphere is not behind ray
            {
                let normal = (point - self.center)/self.radius;
                hit = Some(RaycastHit::new(point, normal, t, self.material));
            }
        }
        return hit;
    }
}
impl SceneObject for Floor
{
    fn raycast(&self, ray: Ray) -> Option<RaycastHit>
    {
        let delta = ray.end - ray.start;
        if delta.y > 0.0
        {
            return None;
        }
        else
        {
            let t = (self.y - ray.start.y) / delta.y;
            let point = delta * t + ray.start;

            return Some(RaycastHit::new(Vec3::new(0.0,1.0,0.0), point, t, self.material));
        }
    }
}



trait SceneObject
{ fn raycast(&self, ray: Ray) -> Option<RaycastHit>; }
