extern crate serde;
use self::serde::Deserialize;
use std::time::{Duration, SystemTime};
use std::ops::Mul;
use std::ops::Add;
use std::ops::Sub;
use std::ops::Neg;
use std::ops::Div;
use std::fmt;



#[derive(Deserialize,Clone,Copy,PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Vec3
    { Vec3{x: x, y: y, z: z} }
    pub fn random() -> Vec3 {
        //this really isn't all that random at all. good enough
        let seed = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
        let seed = format!("{}", seed * seed);
        let x: f64 = seed[seed.len()-3..seed.len()].parse().unwrap();
        let y: f64 = seed[seed.len()-5..seed.len()-2].parse().unwrap();
        let z: f64 = seed[seed.len()-7..seed.len()-4].parse().unwrap();
        Vec3::new(x,y,z).unit()
    }
    pub fn dot(self, other: Vec3) -> f64 {
        self.x * other.x +
        self.y * other.y +
        self.z * other.z
    }
    pub fn cross(self, other: Vec3) -> Vec3 {
        let x = self.y * other.z - self.z * other.y;
        let y = self.z * other.x - self.x * other.z;
        let z = self.x * other.y - self.y * other.x;
        Vec3::new(x,y,z)
    }
    pub fn reflect(self, other: Vec3) -> Vec3 {
        (other * (other.dot(self))) * 2.0 - self
    }
    pub fn magn(&self) -> f64 {
        self.dot(*self).sqrt()
    }
    pub fn unit(self) -> Vec3 {
        self/self.magn()
    }
    pub fn to_color(self) -> Color {
        let r: u8 = (self.x * 128.0 + 128.0) as u8;
        let g: u8 = (self.y * 128.0 + 128.0) as u8;
        let b: u8 = (self.z * 128.0 + 128.0) as u8;
        Color::new(r,g,b,255)
    }
}

impl fmt::Debug for Vec3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("")
         .field("x", &self.x)
         .field("y", &self.y)
         .field("z", &self.z)
         .finish()
    }
}
impl Mul<Vec3> for Vec3 { //this implimentation makes mathmaticians cry
    type Output = Vec3;
    fn mul(self, other: Vec3) -> Vec3 {
        let x = self.x * other.x;
        let y = self.y * other.y;
        let z = self.z * other.z;
        Vec3::new(x,y,z)
    }
}
impl Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, other: f64) -> Vec3 {
        let x = self.x * other;
        let y = self.y * other;
        let z = self.z * other;
        Vec3::new(x,y,z)
    }
}
impl Div<f64> for Vec3 {
    type Output = Vec3;
    fn div(self, other: f64) -> Vec3 {
        let factor = 1.0 / other;
        let x = self.x * factor;
        let y = self.y * factor;
        let z = self.z * factor;
        Vec3::new(x,y,z)
    }
}
impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, other: Vec3) -> Vec3 {
        let x = self.x + other.x;
        let y = self.y + other.y;
        let z = self.z + other.z;
        Vec3::new(x,y,z)
    }
}

#[derive(Clone,Copy,Debug)]
pub struct Matrix3 {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
}
impl Matrix3 {
    pub fn new( a: Vec3, b: Vec3, c: Vec3) -> Matrix3 {
        Matrix3 { a: a, b: b, c: c }
    }
}
impl Mul<Vec3> for Matrix3 {
    type Output = Vec3;
    fn mul(self, other: Vec3) -> Vec3 {
        let x = self.a * other.x;
        let y = self.b * other.y;
        let z = self.c * other.z;
        x + y + z
    }
}

#[derive(Deserialize,Clone,Copy,Debug)]
pub struct Ray {
    pub start: Vec3,
    pub end: Vec3,
}
impl Ray {
    pub fn new(start: Vec3, end: Vec3) -> Ray
    { Ray { start: start, end: end } }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, other: Vec3) -> Vec3 {
        let x = self.x - other.x;
        let y = self.y - other.y;
        let z = self.z - other.z;
        Vec3::new(x,y,z)
    }
}
impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        let x = -self.x;
        let y = -self.y;
        let z = -self.z;
        Vec3::new(x,y,z)
    }
}

#[derive(Deserialize,Clone,Copy,Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}
impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Color
    { Color{ r: r, g: g, b: b, a: a } }
}
impl Add for Color {
    type Output = Color;
    fn add(self, other: Color) -> Color {
        let mut r = self.r as u16 + other.r as u16;
        let mut g = self.g as u16 + other.g as u16;
        let mut b = self.b as u16 + other.b as u16;
        let a = self.a;
        if r > 255
        { r = 255; }
        if g > 255
        { g = 255; }
        if b > 255
        { b = 255; }
        Color::new(r as u8, g as u8, b as u8, a as u8)
    }
}
impl Mul<f64> for Color {
    type Output = Color;
    fn mul(self, other: f64) -> Color {

        let mut r = self.r as f64 * other;
        let mut g = self.g as f64 * other;
        let mut b = self.b as f64 * other;
        if r > 255.0
        { r = 255.0; }
        if g > 255.0
        { g = 255.0; }
        if b > 255.0
        { b = 255.0; }
        Color::new(r as u8, g as u8, b as u8, self.a)
    }
}
#[derive(Deserialize)]
pub struct Camera {
    pub origin: Vec3,
    pub direction: Vec3,
    pub length: f64,
}
#[derive(Deserialize)]
pub struct World {
    pub color: Color,
    pub strength: f64,
}
impl World {
    pub fn new(color: Color, strength: f64) -> World {
        World { color: color, strength: strength }
    }
}


#[derive(Deserialize)]
pub struct PointLight {
    pub origin: Vec3,
    pub strength: f64,
    pub size: f64,
}
impl PointLight {
    pub fn new(origin: Vec3, strength:f64, size:f64) -> PointLight
    { PointLight { origin: origin, strength: strength, size: size } }
}



#[derive(Deserialize)]
#[serde(untagged)]
pub enum Light {
    Point(PointLight),
    Sun(SunLight),
}
//TODO: add directional lighting
#[derive(Deserialize)]
pub struct SunLight {
    //direction: Vec3,
    //strength: f64,
}
#[derive(Deserialize)]
pub struct Tri {
    pub verts: (Vec3,Vec3,Vec3),
    pub vx_normals: (Vec3,Vec3,Vec3),
    pub material: Material
}
impl Tri
{
    pub fn new (
        a: Vec3,    b: Vec3,    c: Vec3, vn_a: Vec3, vn_b: Vec3, vn_c: Vec3, material: Material
    ) -> Tri { 
        Tri { verts: ( a, b, c ), vx_normals: ( vn_a, vn_b, vn_c ), material: material}
    }
    pub fn auto_normal(a: Vec3, b: Vec3, c: Vec3, material: Material) -> Tri { 
        let edge0 = b - a;
        let edge1 = c - a;
        let normal = edge0.cross(edge1).unit();
        Tri { verts: ( a, b, c ), vx_normals: (normal, normal, normal), material: material}
    }
}


#[derive(Deserialize)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f64,
    pub material: Material,
}
impl Sphere {
    pub fn new(center: Vec3, radius: f64, material: Material) -> Sphere
    { Sphere { center: center, radius: radius, material: material } }
}


#[derive(Deserialize,Clone,Copy)]
pub struct Material {
    pub color: Color,
    pub reflective: bool,
}
impl Material {
    pub fn new(color: Color, reflective: bool) -> Material {
        Material { color: color, reflective: reflective }
    }
}


#[derive(Clone,Copy)]
pub struct RaycastHit {
    pub point: Vec3,
    pub normal: Vec3,
    pub depth: f64,
    pub material: Material,
}
impl RaycastHit {
    pub fn new(point: Vec3, normal: Vec3, depth: f64, material: Material) -> RaycastHit
    { RaycastHit { point: point, normal: normal, depth: depth, material: material } }
}

#[derive(Deserialize)]
pub struct Floor {
    pub y: f64,
    pub material: Material
}
impl Floor {
    pub fn new(y: f64, material: Material) -> Floor
    { Floor { y: y, material: material } }
}


