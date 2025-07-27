// /home/pana/Simulations/core-engine/examples/simple_gmsh_test.rs

use core_engine::meshing::generate_mesh_from_geo;
use core_engine::{GeometryDefinition, GeometricPrimitive};

/// A minimal, self-contained example to verify the correct rgmsh API usage.
fn main() {
    let geo_def = GeometryDefinition::Primitive(GeometricPrimitive {
        shape: "cube".to_string(),
        dimensions: vec![1.0, 1.0, 1.0],
    });

    match generate_mesh_from_geo(&geo_def) {
        Ok(mesh) => {
            println!("Successfully generated mesh!");
            println!("  - Element type: {}", mesh.element_type);
            println!("  - Number of nodes: {}", mesh.nodes.len());
            println!("  - Number of elements: {}", mesh.elements.len());
        }
        Err(e) => {
            println!("Error generating mesh: {:?}", e);
        }
    }
}