// src/meshing/mod.rs

//! Handles geometry processing and mesh generation by interfacing with Gmsh.

use crate::{GeometryDefinition, Mesh, EngineError, GeometricPrimitive};
use std::fs;
use std::env;
use std::process::Command;

/// Generates a mesh from a given geometry definition using the gmsh executable.
pub fn generate_mesh_from_geo(geo_def: &GeometryDefinition) -> Result<Mesh, EngineError> {
    let temp_dir = env::temp_dir();
    let output_msh_path = temp_dir.join("temp.msh");
    let output_msh_str = output_msh_path.to_str().ok_or_else(|| EngineError::MeshingFailed("Failed to convert output MSH path to string".to_string()))?;

    let mut command = Command::new("/home/pana/gmsh-4.14.0-Linux64-sdk/bin/gmsh");
    command.arg("-nopopup").arg("-batch");
    command.current_dir(&temp_dir); // Set working directory for Gmsh

    match geo_def {
        GeometryDefinition::File(path) => {
            command.arg(path);
        }
        GeometryDefinition::Primitive(primitive) => {
            let geo_content = create_primitive_geometry(primitive)?;
            let temp_geo_path = temp_dir.join("temp.geo");
            
            fs::write(&temp_geo_path, geo_content.as_bytes())
                .map_err(|e| EngineError::MeshingFailed(format!("Failed to write temp GEO file: {}", e)))?;
            
            // Ensure data is synced to disk
            let file = fs::File::open(&temp_geo_path)
                .map_err(|e| EngineError::MeshingFailed(format!("Failed to open temp GEO file for sync: {}", e)))?;
            file.sync_all()
                .map_err(|e| EngineError::MeshingFailed(format!("Failed to sync temp GEO file: {}", e)))?;

            println!("Wrote GEO content to: {}", temp_geo_path.display());
            println!("Checking GEO file permissions:");
            let ls_output = Command::new("ls").arg("-l").arg(&temp_geo_path).output()
                .map_err(|e| EngineError::MeshingFailed(format!("Failed to run ls command: {}", e)))?;
            println!("ls -l output:\n{}", String::from_utf8_lossy(&ls_output.stdout));
            
                        command.arg("temp.geo"); // Pass relative path since current_dir is set
        }
    }

    command.arg("-3").arg("-o").arg(output_msh_str);

    println!("Running Gmsh command: {:?}", command);
    let output = command.output()
        .map_err(|e| EngineError::MeshingFailed(format!("Failed to execute Gmsh command: {}", e)))?;

    if !output.status.success() {
        return Err(EngineError::MeshingFailed(format!("Gmsh command failed: {}\nStdout: {}\nStderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let mesh = extract_mesh_data_from_file(output_msh_str)?;

    // Clean up temporary files
    if let GeometryDefinition::Primitive(_) = geo_def {
        let temp_geo_path = temp_dir.join("temp.geo");
        let _ = fs::remove_file(&temp_geo_path);
    }
    let _ = fs::remove_file(&output_msh_path);

    Ok(mesh)
}

/// Creates geometry for a primitive shape by generating a .geo file content.
fn create_primitive_geometry(primitive: &GeometricPrimitive) -> Result<String, EngineError> {
    match primitive.shape.as_str() {
        "cube" => {
            if primitive.dimensions.len() != 3 {
                return Err(EngineError::MeshingFailed("Cube requires 3 dimensions [lx, ly, lz]".to_string()));
            }
            let (lx, ly, lz) = (primitive.dimensions[0], primitive.dimensions[1], primitive.dimensions[2]);
            Ok(format!(
                r#"
Point(1) = {{0, 0, 0, 1.0}};
Point(2) = {{{}, 0, 0, 1.0}};
Point(3) = {{{}, {}, 0, 1.0}};
Point(4) = {{0, {}, 0, 1.0}};
Point(5) = {{0, 0, {}, 1.0}};
Point(6) = {{{}, 0, {}, 1.0}};
Point(7) = {{{}, {}, {}, 1.0}};
Point(8) = {{0, {}, {}, 1.0}};

Line(1) = {{1, 2}};
Line(2) = {{2, 3}};
Line(3) = {{3, 4}};
Line(4) = {{4, 1}};
Line(5) = {{5, 6}};
Line(6) = {{6, 7}};
Line(7) = {{7, 8}};
Line(8) = {{8, 5}};
Line(9) = {{1, 5}};
Line(10) = {{2, 6}};
Line(11) = {{3, 7}};
Line(12) = {{4, 8}};

Curve Loop(1) = {{1, 2, 3, 4}};
Plane Surface(1) = {{1}};
Curve Loop(2) = {{5, 6, 7, 8}};
Plane Surface(2) = {{2}};
Curve Loop(3) = {{1, 10, -5, -9}};
Plane Surface(3) = {{3}};
Curve Loop(4) = {{2, 11, -6, -10}};
Plane Surface(4) = {{4}};
Curve Loop(5) = {{3, 12, -7, -11}};
Plane Surface(5) = {{5}};
Curve Loop(6) = {{4, 9, -8, -12}};
Plane Surface(6) = {{6}};

Surface Loop(1) = {{1, 2, 3, 4, 5, 6}};
Volume(1) = {{1}};
                "#,
                lx, ly, lx, ly, lz, lx, lz, lx, ly, lz, ly, lz
            ))
        }
        _ => {
            return Err(EngineError::MeshingFailed(format!("Unsupported primitive shape: {}", primitive.shape)));
        }
    }
}

/// Extracts node and element data from a MSH file into our `Mesh` struct.
fn extract_mesh_data_from_file(file_path: &str) -> Result<Mesh, EngineError> {
    println!("Reading MSH file: {}", file_path);
    let msh_bytes = fs::read(file_path).map_err(|e| EngineError::MeshingFailed(e.to_string()))?;
    println!("Parsing MSH bytes...");
    let msh = mshio::parse_msh_bytes(&msh_bytes).map_err(|e| EngineError::MeshingFailed(e.to_string()))?;
    println!("MSH parsed successfully.");

    let nodes: Vec<[f64; 3]> = msh.data.nodes.unwrap().node_blocks.iter().flat_map(|b| b.nodes.iter()).map(|n| [n.x, n.y, n.z]).collect();
    println!("Extracted {} nodes.", nodes.len());

    let mut elements = Vec::new();
    let mut element_type = "Unknown".to_string();

    if let Some(element_block) = msh.data.elements.unwrap().element_blocks.iter().find(|b| b.element_type == mshio::ElementType::Tet4) {
        element_type = "Tetrahedron".to_string();
        elements = element_block.elements.iter().map(|e| e.nodes.iter().map(|n| *n as usize - 1).collect()).collect(); // Convert to 0-based index
    }

    Ok(Mesh {
        nodes,
        elements,
        element_type,
        boundary_regions: std::collections::HashMap::new(),
    })
}

impl From<i32> for EngineError {
    fn from(err: i32) -> Self {
        EngineError::MeshingFailed(format!("Gmsh error code: {}", err))
    }
}