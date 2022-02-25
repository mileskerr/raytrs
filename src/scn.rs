use std::fs;
use std::sync::Arc;


use crate::*;



pub fn generate_default() -> Scene
{
    let sphere1 = Sphere::new
    (
        Vec3::new(1.0, 0.2, -3.0),
        0.3,
        Material::new(Color::new(0, 0, 255, 255),false),
    );
    let sphere2 = Sphere::new
    (
        Vec3::new(-1.0, 1.0, -4.0),
        0.5,
        Material::new(Color::new(0, 255, 255, 255),false),
    );
    let sphere3 = Sphere::new
    (
        Vec3::new(0.5, 2.0, -2.0),
        0.5,
        Material::new(Color::new(255, 0, 255, 255),false),
    );
    let light1 = PointLight::new
    (
        Vec3::new(0.0, 7.0, -4.0),
        1.0,
    );
    let world = World::new
    (
        Color::new(0, 0, 120, 255),
        1.0,
    );
    let tri1 = Tri::auto_normal
    (
        Vec3::new(1.0,-2.0,7.0),
        Vec3::new(1.0,-2.0,9.0),
        Vec3::new(0.0, -2.0,9.0),
        Material::new(Color::new(255, 0, 0, 255),false),
    );
    //let objects: Vec<Box<dyn SceneObject>> = vec![Box::new(tri1),Box::new(sphere2)];
    let mut objects: Vec<Arc<dyn SceneObject + Send + Sync>> = vec![Arc::new(sphere2),Arc::new(sphere1),Arc::new(sphere3)];
    objects.append(&mut read_obj("teapot1.obj", Material::new(Color::new(255,0,0,255),false)));

    let lights: Vec<Light> = vec![Light::Point(light1)];

    let camera = Camera::new
    (
        //origin
        Vec3::new(0.0, 1.5, -7.0),
        //corners
        Vec3::new(-0.5, 0.5, 1.0),
        Vec3::new(0.5,  0.5, 1.0),
        Vec3::new(-0.5,-0.5, 1.0),
        Vec3::new(0.5, -0.5, 1.0),
    );
    
    Scene::new(objects,lights,camera,world)

}


fn read_obj(filename: &str, material: Material) -> Vec<Arc<dyn SceneObject + Send + Sync>>
{
    println!("\nloading file \"{}\"...",filename);
    let mut verts: Vec<Vec3> = Vec::new();
    let mut norms: Vec<Vec3> = Vec::new();
    let mut tris: Vec<Arc<dyn SceneObject + Send + Sync>> = Vec::new();
    let contents = fs::read_to_string(filename).unwrap();
    {
        for line in contents.lines()
        {
            let is_vert = line.find("v ");
            if is_vert.is_some()
            { 
                let values: Vec<&str> = line.split(' ').collect();
                let x = values[1].parse::<f64>().unwrap();
                let y = values[2].parse::<f64>().unwrap();
                let z = values[3].parse::<f64>().unwrap();
                verts.push(Vec3::new(x,y,z));
            }
            let is_norm = line.find("vn ");
            if is_norm.is_some()
            { 
                let values: Vec<&str> = line.split(' ').collect();
                let x = values[1].parse::<f64>().unwrap();
                let y = values[2].parse::<f64>().unwrap();
                let z = values[3].parse::<f64>().unwrap();
                norms.push(Vec3::new(x,y,z));
            }
            
            let is_face = line.find("f ");
            if is_face.is_some()
            { 
                let values: Vec<&str> = line.split(' ').collect();
                let mut i = Vec::new();
                for value in &values[1..]
                {
                    if value.is_empty() == false
                    {
                        let ind: Vec<&str> = value.split('/').collect();
                        i.push( ind[0].parse::<usize>().unwrap()-1 );
                    }
                }
                let mut n = Vec::new();
                for value in &values[1..]
                {
                    if value.is_empty() == false
                    {
                        let ind: Vec<&str> = value.split('/').collect();
                        n.push( ind[2].parse::<usize>().unwrap()-1 );
                    }
                }
                tris.push( Arc::new(Tri::new(verts[i[0]],verts[i[1]],verts[i[2]],
                                             norms[n[0]],norms[n[1]],norms[n[2]],
                                             material )) );
                if i.len() > 3 //quad
                {
                    tris.push( Arc::new(Tri::new(verts[i[0]],verts[i[2]],verts[i[3]],
                                                 norms[n[0]],norms[n[2]],norms[n[3]],
                                                 material )) );
                }
            }
        }
    }
    return tris;
}

