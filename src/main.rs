use png;
use std::env;
use std::fs::File;
use std::io::BufWriter;
use std::ops::Mul;
use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Sub;
use std::ops::Neg;
use std::ops::Div;
use std::fmt;

const WIDTH:  usize = 512;
const HEIGHT: usize = 512;

fn main() {

    //SCENE//
    
    let sphere1 = Sphere::new
    (
        Vec3::new(2.0, 0.0, 7.0),
        1.5,
        Material::new(Color::new(255, 0, 0, 255),true),
    );
    let sphere2 = Sphere::new
    (
        Vec3::new(-1.5, -0.75, 6.0),
        1.5,
        Material::new(Color::new(0, 255, 255, 255),false),
    );
    let sphere3 = Sphere::new
    (
        Vec3::new(0.5, 0.75, 10.0),
        1.0,
        Material::new(Color::new(255, 255, 0, 255),false),
    );
    let sphere4 = Sphere::new
    (
        Vec3::new(1.0, -1.25, 5.0),
        0.75,
        Material::new(Color::new(0, 0, 255, 255),false),
    );
    let floor = Floor::new
    (
        -2.0,
        Material::new(Color::new(100, 100, 100, 255),false),
    );
    let light1 = PointLight::new
    (
        Vec3::new(-3.0, 2.0, 5.0),
        1.0,
    );
    let light2 = PointLight::new
    (
        Vec3::new(4.0, 2.0, 2.0),
        1.0,
    );
    let world = World::new
    (
        Color::new(0, 0, 20, 255),
        1.0,
    );
    let objects: Vec<Box<dyn SceneObject>> = vec![Box::new(floor),Box::new(sphere3),Box::new(sphere2),Box::new(sphere1),Box::new(sphere4)];
    let lights: Vec<Light> = vec![Light::Point(light2),Light::Point(light1)];
    let camera = Camera::new
    (
        //origin
        Vec3::new(0.0, 0.0, 0.0),
        //corners
        Vec3::new(-0.5, 0.5, 1.0),
        Vec3::new(0.5,  0.5, 1.0),
        Vec3::new(-0.5,-0.5, 1.0),
        Vec3::new(0.5, -0.5, 1.0),
    );
    let scene = Scene::new(objects,lights,camera,world);
    
   

    //OUTPUT

    let pixels = scene.render();

    let path = env::args()
        .nth(1)
        .expect("Expected a filename to output to.");
    let file = File::create(path).unwrap();
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


struct Scene
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
        let mut pixels: Vec<Color> = Vec::new();
        
        let rays = &self.camera.rays();

        let num_rays = rays.len();
        for _ in 0..num_rays
        { pixels.push(self.world.color); }
        
        for object_index in 0..self.objects.len()
        {
            for i in 0..num_rays
            {
                let hit = &self.objects[object_index].raycast(rays[i]);
                if hit.is_some()
                {
                    if hit.unwrap().material.reflective
                    {
                        pixels[i] = self.shade_reflective( rays[i],hit.unwrap());
                    }
                    else
                    {
                        pixels[i] = self.shade_diffuse(hit.unwrap());
                    }
                }
            }
        }
        return pixels;
    }
    fn shade_diffuse(&self, hit: RaycastHit) -> Color
    {
        let mut pixel = self.world.color * self.world.strength;
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
                    let mut l0 = light_dir.dot(hit.normal);
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
        pixel = hit.material.color * lightness;
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
                pixel = self.shade_diffuse( hit1.unwrap() )
            }
        }
        return pixel;
    }
}


struct Camera
{
    origin: Vec3,
    upper_left: Vec3,
    upper_right: Vec3,
    lower_left: Vec3,
    lower_right: Vec3,
    far_clip: f64,
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
                let ray_end = (dir * self.far_clip) + self.origin;
                output.push(Ray::new(self.origin, ray_end));
                dir.x += dx;
            }
            dir.y -= dy;
        }
        return output;
    }
}

struct World
{
    color: Color,
    strength: f64,
}
impl World
{
    fn new(color: Color, strength: f64) -> World
    {
        World { color: color, strength: strength }
    }
}

enum Light
{
    Point(PointLight),
    Sun(SunLight),
}

struct PointLight
{
    origin: Vec3,
    strength: f64,
}
impl PointLight
{
    fn new(origin: Vec3, strength:f64) -> PointLight
    { PointLight { origin: origin, strength: strength } }
}

struct SunLight
{
    direction: Vec3,
    strength: f64,
}

struct Sphere
{
    center: Vec3,
    radius: f64,
    material: Material,
}
impl Sphere
{
    fn new(center: Vec3, radius: f64, material: Material) -> Sphere
    { Sphere { center: center, radius: radius, material: material } }
}
impl SceneObject for Sphere
{
    fn raycast(&self, ray: Ray) -> Option<RaycastHit>
    {
        let delta = ray.end - ray.start;
       
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
            let point = ray.start + ( Vec3::new(t * delta.x, t * delta.y, t * delta.z ) );
            if (point - ray.start).dot(delta) > 0.0 //check that sphere is not behind ray
            {
                let normal = (point - self.center) * (1.0/self.radius);
                hit = Some(RaycastHit 
                {
                    point: point,
                    normal: normal,
                    material: self.material,
                });
            }
        }
        return hit;
    }
}
#[derive(Clone,Copy)]
struct Material
{
    color: Color,
    reflective: bool,
}
impl Material
{
    fn new(color: Color, reflective: bool) -> Material
    {
        Material { color: color, reflective: reflective }
    }
}
#[derive(Clone,Copy)]
struct RaycastHit
{
    point: Vec3,
    normal: Vec3,
    material: Material,
}
struct Floor
{
    y: f64,
    material: Material
}
impl Floor
{
    fn new(y: f64, material: Material) -> Floor
    { Floor { y: y, material: material } }
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
            println!("{:?}",point);

            return Some(RaycastHit
            {
                normal: Vec3::new(0.0,1.0,0.0),
                point: point,
                material: self.material,
            });
        }
    }
}


#[derive(Clone,Copy,Debug)]
struct Ray
{
    start: Vec3,
    end: Vec3,
}
impl Ray
{
    fn new(start: Vec3, end: Vec3) -> Ray
    { Ray { start: start, end: end } }
}

#[derive(Clone,Copy,PartialEq)]
struct Vec3
{
    x: f64,
    y: f64,
    z: f64,
}
impl Vec3
{
    fn new(x: f64, y: f64, z: f64) -> Vec3
    { Vec3{x: x, y: y, z: z} }
    fn dot(self, other: Vec3) -> f64
    {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
    fn reflect(self, other: Vec3) -> Vec3
    {
        (other.unit() * (other.dot(self))) * 2.0 - self
    }
    fn magn(&self) -> f64
    {
        self.dot(*self).sqrt()
    }
    fn unit(self) -> Vec3
    {
        self/self.magn()
    }
    fn to_color(self) -> Color
    {
        let r: u8 = (self.x * 128.0 + 128.0) as u8;
        let g: u8 = (self.y * 128.0 + 128.0) as u8;
        let b: u8 = (self.z * 128.0 + 128.0) as u8;

        Color::new(r,g,b,255)
    }

}
impl fmt::Debug for Vec3
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.debug_struct("")
         .field("x", &self.x)
         .field("y", &self.y)
         .field("z", &self.z)
         .finish()
    }
}
impl Mul<f64> for Vec3
{
    type Output = Vec3;
    fn mul(self, other: f64) -> Vec3
    {
        let x = self.x * other;
        let y = self.y * other;
        let z = self.z * other;
        Vec3::new(x,y,z)
    }
}
impl Div<f64> for Vec3
{
    type Output = Vec3;
    fn div(self, other: f64) -> Vec3
    {
        let x = self.x / other;
        let y = self.y / other;
        let z = self.z / other;
        Vec3::new(x,y,z)
    }
}
impl Add for Vec3
{
    type Output = Vec3;
    fn add(self, other: Vec3) -> Vec3
    {
        let x = self.x + other.x;
        let y = self.y + other.y;
        let z = self.z + other.z;
        Vec3::new(x,y,z)
    }
}
impl Sub for Vec3
{
    type Output = Vec3;
    fn sub(self, other: Vec3) -> Vec3
    {
        let x = self.x - other.x;
        let y = self.y - other.y;
        let z = self.z - other.z;
        Vec3::new(x,y,z)
    }
}
impl Neg for Vec3
{
    type Output = Vec3;
    fn neg(self) -> Vec3
    {
        let x = -self.x;
        let y = -self.y;
        let z = -self.z;
        Vec3::new(x,y,z)
    }
}


#[derive(Clone,Copy,Debug)]
struct Color
{
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}
impl Color
{
    fn new(r: u8, g: u8, b: u8, a: u8) -> Color
    { Color{ r: r, g: g, b: b, a: a } }
}
impl Add for Color
{
    type Output = Color;
    fn add(self, other: Color) -> Color
    {
        let mut r = self.r as u16 + other.r as u16;
        let mut g = self.g as u16 + other.g as u16;
        let mut b = self.b as u16 + other.b as u16;
        let mut a = self.a;
        if r > 255
        { r = 255; }
        if g > 255
        { g = 255; }
        if b > 255
        { b = 255; }
        Color::new(r as u8, g as u8, b as u8, a as u8)
    }
}
impl Mul<f64> for Color
{
    type Output = Color;
    fn mul(self, other: f64) -> Color {

        let mut r = (self.r as f64 * other);
        let mut g = (self.g as f64 * other);
        let mut b = (self.b as f64 * other);
        if r > 255.0
        { r = 255.0; }
        if g > 255.0
        { g = 255.0; }
        if b > 255.0
        { b = 255.0; }
        Color::new(r as u8, g as u8, b as u8, self.a)
    }
}



trait SceneObject
{
    fn raycast(&self, ray: Ray) -> Option<RaycastHit>;
}

