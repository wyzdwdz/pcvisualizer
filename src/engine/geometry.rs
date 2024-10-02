use std::rc::Rc;

use bytemuck::{Pod, Zeroable};

pub struct Cylinder {
    objects: Vec<Rc<Object>>,
}

impl Cylinder {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
        }
    }

    pub fn add_cylinder(&mut self) -> Rc<Object> {
        let vertices = vec![];
        let indices = vec![];

        let object = Rc::new(Object::new(vertices, indices));
        self.objects.push(object.clone());

        object.clone()
    }
}

pub struct Object {
    vertices: Box<[Vertex]>,
    indices: Box<[u32]>,
}

impl Object {
    fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        let vertices = vertices.as_slice();
        let indices = indices.as_slice();

        Self {
            vertices: vertices.into(),
            indices: indices.into(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
}
