use ash::vk;
use glam::{Vec2, Vec3};
use image::{EncodableLayout, GenericImage};
use thunderdome::Index;

use crate::{
    renderer::{renderer::VulkanContext, scene3d::MeshVertex, textures::SkyboxImageData},
    scene::MeshData,
};

pub const ASSET_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../assets");

pub fn process_gltf_mesh(mesh: &gltf::Mesh, buffers: &[gltf::buffer::Data]) -> MeshData {
    let mut mesh_vertices = Vec::new();
    let mut mesh_indices = Vec::new();

    let prim = mesh.primitives().next().unwrap();
    let reader = prim.reader(|b| Some(&buffers[b.index()]));

    let mut positions = reader.read_positions().unwrap();
    let mut normals = reader.read_normals().unwrap();
    let mut uvs = reader.read_tex_coords(0).unwrap().into_f32();
    let indices = reader.read_indices().unwrap().into_u32();

    let v_count = positions.len();

    for _ in 0..v_count {
        let position = positions.next().unwrap();
        let normal = normals.next().unwrap();
        let uv = uvs.next().unwrap();

        mesh_vertices.push(MeshVertex {
            pos: Vec3::from_slice(&position),
            norm: Vec3::from_slice(&normal),
            uv: Vec2::from_slice(&uv),
        });
    }

    for index in indices {
        mesh_indices.push(index as u16);
    }

    MeshData {
        vertices: mesh_vertices,
        indices: mesh_indices,
    }
}

pub fn get_first_gltf_mesh(filename: &str) -> MeshData {
    let (gltf, buffers, _images) = gltf::import(filename).unwrap();

    let mesh = gltf.meshes().next().unwrap();
    process_gltf_mesh(&mesh, &buffers)
}

pub fn load_skybox<'a>(render: &mut VulkanContext, file_path: &str) -> Index {
    let mut skybox_image = image::open(file_path).unwrap().into_rgba8();

    let top_image = skybox_image.sub_image(512, 0, 512, 512).to_image();
    let left_image = skybox_image.sub_image(0, 512, 512, 512).to_image();
    let back_image = skybox_image.sub_image(512, 512, 512, 512).to_image();
    let right_image = skybox_image.sub_image(512 * 2, 512, 512, 512).to_image();
    let front_image = skybox_image.sub_image(512 * 3, 512, 512, 512).to_image();
    let bottom_image = skybox_image.sub_image(512, 512 * 2, 512, 512).to_image();

    let skybox_data = SkyboxImageData {
        width: 512,
        height: 512,
        top: top_image.as_bytes(),
        bottom: bottom_image.as_bytes(),
        front: front_image.as_bytes(),
        back: back_image.as_bytes(),
        left: left_image.as_bytes(),
        right: right_image.as_bytes(),
    };

    render.load_skybox(vk::Filter::LINEAR, &skybox_data)
}
