use png;
use std::env;
use std::fs::File;
use std::time::Instant;
use std::io::BufWriter;

mod scn;
mod space;

pub use space::*;

const WIDTH:  usize = 256;
const HEIGHT: usize = 256;

fn main()
{
    let path = env::args()
        .nth(1)
        .expect("Expected a filename to output to.");

    let scene = scn::generate_default();    
    let pixels = scene.render();

    write_file(pixels, path);

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
    objects: Vec<Box<dyn SceneObject>>,
    lights: Vec<Light>,
    camera: Camera,
    world: World,
}
impl Scene
{
    fn new(objects: Vec<Box<dyn SceneObject>>, lights: Vec<Light>, camera: Camera, world: World) -> Scene
    {
        Scene { objects: objects, lights: lights, camera: camera, world: world }
    }
    fn render(&self) -> Vec<Color>
    {
        let t0 = Instant::now();

        let mut pixels: Vec<Color> = Vec::new();
        let mut depth_buffer: Vec<f64> = Vec::new();
        
        println!("getting view rays...");
        let rays = &self.camera.rays();

        let num_rays = rays.len();
        for _ in 0..num_rays
        {
            pixels.push(self.world.color);
            depth_buffer.push(1000.0);
        }

        let num_objects = self.objects.len();

        println!("\nrendering...");
        print!("objects:  ");
        
        for object_index in 0..num_objects
        {
            let percent = ((object_index as f32 /num_objects as f32) * 100.0) as u8;
            print!("\x1b[s{}/{}\x1b[u",object_index,num_objects);
            for i in 0..num_rays
            {

                let hit = &self.objects[object_index].raycast(rays[i]);
                if hit.is_some()
                {
                    if hit.unwrap().depth < depth_buffer[i]
                    {
                        depth_buffer[i] = hit.unwrap().depth;
                        if hit.unwrap().material.reflective
                        {
                            pixels[i] = self.shade_reflective( rays[i],hit.unwrap());
                        }
                        else
                        {
                            //pixels[i] = hit.unwrap().normal.to_color();
                            pixels[i] = self.shade_diffuse(hit.unwrap());
                        }
                    }
                }
            }
        }

        println!("finished rendering in {} secs", t0.elapsed().as_secs());
        return pixels;
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
                    for object_index_1 in 0..self.objects.len()
                    {
                        let ray = Ray::new( hit.point, point_light.origin);
                        let hit1 = &self.objects[object_index_1].raycast( ray );
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
    fn rays(&self) -> Vec<Ray>
    {
        let mut output = Vec::new();
        let mut dir = self.upper_left;

        let dx = (self.upper_right.x - self.upper_left.x)/(WIDTH as f64);
        let dy = (self.upper_right.y - self.lower_right.y)/(HEIGHT as f64);

        for _y in 0..HEIGHT
        {
            dir.x = self.upper_left.x;
            for _x in 0..WIDTH
            {
                let ray_end = dir + self.origin;
                output.push(Ray::new(self.origin, ray_end));
                dir.x += dx;
            }
            dir.y -= dy;
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
            return Some(RaycastHit
            {
                point: point,
                depth: t,
                normal: self.normal,
                material: self.material,
            });
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
            let t = (-b -dsc.sqrt()) / (2.0 * a);
            let point = ray.start + (delta * t);
            if (point - ray.start).dot(delta) > 0.0 //check that sphere is not behind ray
            {
                let normal = (point - self.center)/self.radius;
                hit = Some(RaycastHit 
                {
                    point: point,
                    depth: t,
                    normal: normal,
                    material: self.material,
                });
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
            let scl = (self.y - ray.start.y) / delta.y;
            let point = delta * scl + ray.start;

            return Some(RaycastHit
            {
                normal: Vec3::new(0.0,1.0,0.0),
                point: point,
                depth: scl,
                material: self.material,
            });
        }
    }
}



trait SceneObject
{
    fn raycast(&self, ray: Ray) -> Option<RaycastHit>;
}
