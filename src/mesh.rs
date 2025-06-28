use crate::file_ext::*;
use crate::rsz::*;
use crate::reerr::{Result, FileParseError::*};
use std::io::{Read, Seek};
use serde::Serialize;

pub type Vertex = [f32; 3];


#[derive(Debug, Clone)]
pub struct Subdivision {
    pub material_index: u8,
    pub is_quad: u8,
    pub vertex_buffer_index: u8,
    pub index_count: u32,
    pub index_start_index: u32,
    pub vertex_start_index: u32,
}

impl Subdivision {
    pub fn read<F: Read + Seek>(file: &mut F) -> Result<Self> {
        let material_index = file.read_u8()?;
        let is_quad = file.read_u8()?;
        let vertex_buffer_index = file.read_u8()?;
        let _padding = file.read_u8()?;
        let _ = file.read_u32()?;
        let index_count = file.read_u32()?; 
        let index_start_index = file.read_u32()?; 
        let vertex_start_index = file.read_u32()?; 
        let _ = file.read_u32()?; // stremaing bytes stuff
        let _ = file.read_u32()?;
        let _ = file.read_u32()?;
        Ok(Self {
            material_index,
            is_quad,
            vertex_buffer_index,
            vertex_start_index,
            index_start_index,
            index_count
        })
    }
}

#[derive(Debug, Clone)]
pub struct MeshGroup {
    pub group_id: u8,
    pub subdivisions: Vec<Subdivision>,
}

impl MeshGroup {
    pub fn read<F: Read + Seek>(file: &mut F) -> Result<Self> {
        let group_id = file.read_u8()?;
        let mesh_count = file.read_u8()?;
        let _null1 = file.read_u16()?;
        let _null2 = file.read_u16()?;
        let _null3 = file.read_u16()?;
        let _vertex_count = file.read_u32()?;
        let _index_count = file.read_u32()?;
        let subdivisions = (0..mesh_count).map(|_| {
            Subdivision::read(file)
        }).collect::<Result<Vec<Subdivision>>>()?;
        Ok(Self {
            group_id,
            subdivisions
        })
    }
}

#[derive(Debug, Clone)]
pub struct LodGroup {
    pub mesh_groups: Vec<MeshGroup>,
}

impl LodGroup {
    pub fn read<F: Read + Seek>(file: &mut F) -> Result<Self> {
        let count = file.read_u8()?;
        let _vertex_format = file.read_u8()?;
        let _reserved = file.read_u16()?;
        let _distance = file.read_f32()?;
        let _offset_offset = file.read_u64()?;
        let mesh_group_offsets = (0..count).map(|_| { file.read_u64() }).collect::<Result<Vec<u64>>>()?;
        file.seek_align_up(16)?;
        let mesh_groups = mesh_group_offsets.iter().map(|&offset| {
            file.seek(std::io::SeekFrom::Start(offset))?;
            MeshGroup::read(file)
        }).collect::<Result<Vec<_>>>()?;
        Ok(Self {
            mesh_groups
        })
    }
}

#[derive(Debug)]
pub struct VertexLayout {
    pub typing: u16,
    pub stride: u16,
    pub pos_start_offset: u32
}

impl VertexLayout {
    pub fn read<F: Read + Seek>(file: &mut F) -> Result<Self> {
        Ok(Self {
            typing: file.read_u16()?,
            stride: file.read_u16()?,
            pos_start_offset: file.read_u32()?
        })
    }
}

#[derive(Debug)]
pub struct Mesh {
    version: u32,
    pub vertex_buffer: Vec<u8>,
    pub vertex_elements: Vec<VertexLayout>,
    pub index_buffer: Vec<u8>,
    pub total_buffer_size: u32,
    pub lod_groups: Vec<LodGroup>,
    pub names: Vec<String>,
}

impl Mesh {
    pub fn new<F: Read + Seek>(mut file: F) -> Result<Self> {
        let magic = file.read_magic()?;
        let ext = core::str::from_utf8(&magic)?;
        if ext != "MESH" {
            return Err(Box::new(MagicError { 
                real_magic: String::from("MESH"), 
                read_magic: ext.to_string()
            }))
        }
        let version = file.read_u32()?;
        let file_size = file.read_u32()?;
        let unkn_hash = file.read_u32()?;
        // read content flag
        let wilds_unkn1 = file.read_u32()?;
        let name_count = file.read_u16()?;
        let content_flag = file.read_u16()?;
        let sf6_unkn_count = file.read_u16()?;
        //println!("version:{version} name_count:{name_count}, content_flag:{content_flag:b}");

        let wilds_unkn2 = file.read_u32()?;
        let wilds_unkn3 = file.read_u32()?;
        let wilds_unkn4 = file.read_u32()?;
        let wilds_unkn5 = file.read_u16()?;

        let vertices_offset = file.read_u64()?;
        let mesh_group_offset = file.read_u64()?;
        let shadow_mesh_group_offset = file.read_u64()?;
        let occlusion_mesh_group_offset = file.read_u64()?;
        let normal_recalc_offset = file.read_u64()?;
        let blend_shape_offset = file.read_u64()?;
        let mesh_offset = file.read_u64()?;
        let sf6_unkn1 = file.read_u64()?;
        //println!("mesh_offset {mesh_offset:x}, vertices_offset {vertices_offset:x}");
        let floats_offset = file.read_u64()?;
        let aabb_offset = file.read_u64()?;
        let skeleton_offset = file.read_u64()?;
        let material_name_remap_offset = file.read_u64()?;
        let bone_name_remap_offset = file.read_u64()?;
        let blend_shape_name_offset = file.read_u64()?;
        let name_offsets_offset = file.read_u64()?;
        let streaming_info_offset = file.read_u64()?;
        let sf6_unkn4 = file.read_u64()?;

        /*
         * read lods
         */
        file.seek(std::io::SeekFrom::Start(mesh_group_offset))?;
        let lod_group_count = file.read_u8()?;
        let material_count = file.read_u8()?;
        let uv_count = file.read_u8()?;
        let skin_weight_count = file.read_u8()?;
        let total_mesh_count = file.read_u16()?;
        let has_32bitindexbuffer = file.read_u8()?;
        let shared_lod_bits = file.read_u8()?;
        //let _ = file.read_u64()?;
        let sphere = file.read_f32vec4()?;
        let aabb = (file.read_f32vec4()?, file.read_f32vec4()?);
        let offset_offset = file.read_u64()?;
        let lod_group_offsets = (0..lod_group_count).map(|_| { file.read_u64() }).collect::<Result<Vec<u64>>>()?;
        let lod_groups = lod_group_offsets.iter().map(|&offset| {
            file.seek(std::io::SeekFrom::Start(offset))?;
            LodGroup::read(&mut file)
        }).collect::<Result<Vec<_>>>()?;
        //println!("{:#?}", lod_groups);


        /*
         * read the mesh stuff
         */
        file.seek(std::io::SeekFrom::Start(mesh_offset))?;
        let vertex_element_offset = file.read_u64()?;
        let vertex_buffer_offset = file.read_u64()?;

        let sunbreakoffset = file.read_u64()?;
        let total_buffer_size = file.read_u32()?;
        let vertex_buffer_size = file.read_u32()?;
        let main_vertex_element_count = file.read_u16()?;
        let vertex_element_count = file.read_u16()?;
        let block_2_face_buffer_offset = file.read_u32()?;
        let unk = file.read_u32()?;
        //println!("PLEASE {unk}, {block_2_face_buffer_offset}, {main_vertex_element_count}");
        let vertex_element_size = file.read_u16()?;
        let unkn1 = file.read_u16()?;
        let sunbreak_unk2 = file.read_u64()?;
        let sf6_unkn0 = file.read_u64()?;
        let streaming_vertex_element_offset = file.read_u64()?;
        let sf6_unkn2 = file.read_u64()?;

        //read vertex elements
        file.seek(std::io::SeekFrom::Start(vertex_element_offset))?;
        let vertex_elements = (0..vertex_element_count).map(|_| {
            VertexLayout::read(&mut file)
        }).collect::<Result<Vec<_>>>()?;
        // read vertex buffer 
        file.seek(std::io::SeekFrom::Start(vertex_buffer_offset))?;
        let vertex_buffer = file.read_u8_n(vertex_buffer_size as usize)?;
        file.seek(std::io::SeekFrom::Start(vertex_buffer_offset + vertex_buffer_size as u64))?;
        let index_buffer = file.read_u8_n((block_2_face_buffer_offset - vertex_buffer_size) as usize)?;

        /*println!("{vertex_buffer_size:x}, {vertex_element_size}, stremaing_info {streaming_info_offset:x}");
        println!("index_buf info {}", block_2_face_buffer_offset - vertex_buffer_size);
        println!("index_buf info2 {}", total_buffer_size - vertex_buffer_size);
        println!("LEN{}", index_buffer.len());*/
        

        /*
         * Read Names
         */
        file.seek(std::io::SeekFrom::Start(name_offsets_offset))?;
        let name_offsets = (0..name_count).map(|_| { file.read_u64() }).collect::<Result<Vec<_>>>()?;
        let names = name_offsets.iter().map(|&offset| { 
            file.seek(std::io::SeekFrom::Start(offset))?;
            file.read_u8str()
        }).collect::<Result<Vec<_>>>()?;
        //println!("{:?}", names);


        Ok(Self {
            version,
            vertex_buffer,
            vertex_elements,
            index_buffer,
            total_buffer_size,
            lod_groups,
            names,
        })
    }
}
