use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coord: [f32; 2],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MeshType {
    Triangles,
    HorizontalLines,
    VerticalLines,
    Grid,
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub mesh_type: MeshType,
}

impl Mesh {
    pub fn triangle_mesh(grid_size: u32, width: f32, height: f32) -> Self {
        let mut vertices = Vec::new();
        let rescale = 1.0 / grid_size as f32;

        for i in 0..grid_size {
            for j in 0..grid_size {
                let x0 = j as f32 * width / grid_size as f32;
                let x1 = (j + 1) as f32 * width / grid_size as f32;
                let y0 = i as f32 * height / grid_size as f32;
                let y1 = (i + 1) as f32 * height / grid_size as f32;

                let tex_x0 = j as f32 * rescale;
                let tex_x1 = (j + 1) as f32 * rescale;
                let tex_y0 = i as f32 * rescale;
                let tex_y1 = (i + 1) as f32 * rescale;

                // First triangle
                vertices.push(Vertex {
                    position: [x0, y0, 0.0],
                    tex_coord: [tex_x0, tex_y0],
                });
                vertices.push(Vertex {
                    position: [x1, y0, 0.0],
                    tex_coord: [tex_x1, tex_y0],
                });
                vertices.push(Vertex {
                    position: [x1, y1, 0.0],
                    tex_coord: [tex_x1, tex_y1],
                });

                // Second triangle
                vertices.push(Vertex {
                    position: [x1, y1, 0.0],
                    tex_coord: [tex_x1, tex_y1],
                });
                vertices.push(Vertex {
                    position: [x0, y1, 0.0],
                    tex_coord: [tex_x0, tex_y1],
                });
                vertices.push(Vertex {
                    position: [x0, y0, 0.0],
                    tex_coord: [tex_x0, tex_y0],
                });
            }
        }

        Self {
            vertices,
            mesh_type: MeshType::Triangles,
        }
    }

    pub fn horizontal_line_mesh(grid_size: u32, width: f32, height: f32) -> Self {
        let new_grid_size = grid_size * 2;
        let mut vertices = Vec::new();
        let rescale = 1.0 / new_grid_size as f32;

        for i in 0..new_grid_size {
            for j in 0..new_grid_size {
                let x0 = j as f32 * width / new_grid_size as f32;
                let x1 = (j + 1) as f32 * width / new_grid_size as f32;
                let y0 = i as f32 * height / new_grid_size as f32;

                let tex_x0 = j as f32 * rescale;
                let tex_x1 = (j + 1) as f32 * rescale;
                let tex_y0 = i as f32 * rescale;

                vertices.push(Vertex {
                    position: [x0, y0, 0.0],
                    tex_coord: [tex_x0, tex_y0],
                });
                vertices.push(Vertex {
                    position: [x1, y0, 0.0],
                    tex_coord: [tex_x1, tex_y0],
                });
            }
        }

        Self {
            vertices,
            mesh_type: MeshType::HorizontalLines,
        }
    }

    pub fn vertical_line_mesh(grid_size: u32, width: f32, height: f32) -> Self {
        let new_grid_size = grid_size * 2;
        let mut vertices = Vec::new();
        let rescale = 1.0 / new_grid_size as f32;

        for i in 0..new_grid_size {
            for j in 0..new_grid_size {
                let x0 = i as f32 * width / new_grid_size as f32;
                let y0 = j as f32 * height / new_grid_size as f32;
                let y1 = (j + 1) as f32 * height / new_grid_size as f32;

                let tex_x0 = i as f32 * rescale;
                let tex_y0 = j as f32 * rescale;
                let tex_y1 = (j + 1) as f32 * rescale;

                vertices.push(Vertex {
                    position: [x0, y0, 0.0],
                    tex_coord: [tex_x0, tex_y0],
                });
                vertices.push(Vertex {
                    position: [x0, y1, 0.0],
                    tex_coord: [tex_x0, tex_y1],
                });
            }
        }

        Self {
            vertices,
            mesh_type: MeshType::VerticalLines,
        }
    }

    /// Grid mesh - combines horizontal and vertical lines for wireframe effect
    pub fn grid_mesh(grid_size: u32, width: f32, height: f32) -> Self {
        let new_grid_size = grid_size * 2;
        let mut vertices = Vec::new();
        let rescale = 1.0 / new_grid_size as f32;

        // Horizontal lines
        for i in 0..new_grid_size {
            for j in 0..new_grid_size {
                let x0 = j as f32 * width / new_grid_size as f32;
                let x1 = (j + 1) as f32 * width / new_grid_size as f32;
                let y0 = i as f32 * height / new_grid_size as f32;

                let tex_x0 = j as f32 * rescale;
                let tex_x1 = (j + 1) as f32 * rescale;
                let tex_y0 = i as f32 * rescale;

                vertices.push(Vertex {
                    position: [x0, y0, 0.0],
                    tex_coord: [tex_x0, tex_y0],
                });
                vertices.push(Vertex {
                    position: [x1, y0, 0.0],
                    tex_coord: [tex_x1, tex_y0],
                });
            }
        }

        // Vertical lines
        for i in 0..new_grid_size {
            for j in 0..new_grid_size {
                let x0 = i as f32 * width / new_grid_size as f32;
                let y0 = j as f32 * height / new_grid_size as f32;
                let y1 = (j + 1) as f32 * height / new_grid_size as f32;

                let tex_x0 = i as f32 * rescale;
                let tex_y0 = j as f32 * rescale;
                let tex_y1 = (j + 1) as f32 * rescale;

                vertices.push(Vertex {
                    position: [x0, y0, 0.0],
                    tex_coord: [tex_x0, tex_y0],
                });
                vertices.push(Vertex {
                    position: [x0, y1, 0.0],
                    tex_coord: [tex_x0, tex_y1],
                });
            }
        }

        Self {
            vertices,
            mesh_type: MeshType::Grid,
        }
    }

    pub fn primitive_topology(&self) -> wgpu::PrimitiveTopology {
        match self.mesh_type {
            MeshType::Triangles => wgpu::PrimitiveTopology::TriangleList,
            MeshType::HorizontalLines | MeshType::VerticalLines | MeshType::Grid => wgpu::PrimitiveTopology::LineList,
        }
    }
}
