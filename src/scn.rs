use std::fs;


use crate::*;



pub fn generate_default() -> Scene
{
    let sphere1 = Sphere::new
    (
        Vec3::new(-1.2, 0.5, -1.0),
        0.5,
        Material::new(Color::new(0, 0, 255, 255),false),
    );
    let sphere2 = Sphere::new
    (
        Vec3::new(-1.0, 0.75, -4.0),
        0.75,
        Material::new(Color::new(0, 255, 255, 255),false),
    );
    let sphere3 = Sphere::new
    (
        Vec3::new(0.5, 0.5, -3.0),
        0.5,
        Material::new(Color::new(255, 0, 255, 255),false),
    );
    let sphere4 = Sphere::new
    (
        Vec3::new(3.0, 1.0, -2.0),
        1.0,
        Material::new(Color::new(255, 255, 0, 255),false),
    );
    let sphere5 = Sphere::new
    (
        Vec3::new(0.5, 4.0, 5.0),
        2.5,
        Material::new(Color::new(0, 255, 0, 255),false),
    );
    let light1 = PointLight::new
    (
        Vec3::new(0.0, 7.0, -4.0),
        1.0,
    );
    let floor = Floor::new
    (
        0.0,
        Material::new(Color::new(255,255,255,255),false),
    );
    let world = World::new
    (
        Color::new(0, 0, 0, 255),
        1.0,
    );
    let mut objects: Vec<Box<dyn SceneObject + Send + Sync>> = vec![Box::new(floor),Box::new(sphere2),Box::new(sphere1),Box::new(sphere3),Box::new(sphere4),Box::new(sphere5)];
    objects.append(&mut read_obj("teapot1.obj", Material::new(Color::new(100,100,100,255),true)));

    let lights: Vec<Light> = vec![Light::Point(light1)];

    let camera = Camera::new
    (
        //origin
        Vec3::new(0.0, 3.0, -7.0),
        //direction
        Vec3::new(0.0, -0.2, 1.0),
        //focal length
        1.0,
    );
    
    Scene::new(objects,lights,camera,world)

}


fn read_obj(filename: &str, material: Material) -> Vec<Box<dyn SceneObject + Send + Sync>>
{
    println!("loading file \"{}\"...",filename);
    let mut verts: Vec<Vec3> = Vec::new();
    let mut norms: Vec<Vec3> = Vec::new();
    let mut tris: Vec<Box<dyn SceneObject + Send + Sync>> = Vec::new();
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
                tris.push( Box::new(Tri::new(verts[i[0]],verts[i[1]],verts[i[2]],
                                             norms[n[0]],norms[n[1]],norms[n[2]],
                                             material )) );
                if i.len() > 3 //quad
                {
                    tris.push( Box::new(Tri::new(verts[i[0]],verts[i[2]],verts[i[3]],
                                                 norms[n[0]],norms[n[2]],norms[n[3]],
                                                 material )) );
                }
            }
        }
    }
    return tris;
}

