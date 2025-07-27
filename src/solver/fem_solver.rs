// src/solver/fem_solver.rs

//! A basic Finite Element Method (FEM) solver.

use crate::{ProblemDefinition, EngineError, Mesh, Material};
use crate::solver::Solver;
use nalgebra::{DMatrix, DVector};

/// A simple FEM solver for linear elasticity.
///
/// This solver calculates nodal displacements for a given mesh under specified
/// boundary conditions and material properties.
pub struct FemSolver;

impl Solver for FemSolver {
    fn name(&self) -> &'static str {
        "FemSolver"
    }

    fn solve(&self, problem: &mut ProblemDefinition) -> Result<super::SolverSolutionData, EngineError> {
        println!("--- Running FemSolver (Linear Elasticity) ---");

        let mesh = problem.mesh.as_ref().ok_or_else(|| EngineError::SolverFailed("Mesh not found in problem definition".to_string()))?;
        let material = &problem.physics.material;

        if mesh.element_type != "Tetrahedron" {
            return Err(EngineError::SolverFailed(format!("FemSolver currently only supports Tetrahedral meshes, but found {}", mesh.element_type)));
        }

        // 1. Initialize global stiffness matrix (K) and force vector (F).
        let num_nodes = mesh.nodes.len();
        let dof_per_node = 3; // 3 degrees of freedom (x, y, z displacement) per node
        let total_dof = num_nodes * dof_per_node;

        let mut k_global = DMatrix::<f64>::zeros(total_dof, total_dof);
        let mut f_global = DVector::<f64>::zeros(total_dof);

        // 2. Assemble element stiffness matrices and global system.
        // For simplicity, we'll assume a constant strain tetrahedron (CST) for now.
        // This is a placeholder for the actual element stiffness matrix assembly.
        // In a real FEM solver, this would involve complex matrix algebra.
        for (elem_idx, element) in mesh.elements.iter().enumerate() {
            if element.len() != 4 {
                return Err(EngineError::SolverFailed(format!("Element {} is not a tetrahedron (node count: {})", elem_idx, element.len())));
            }

            // Get node coordinates for the current element.
            let n1 = mesh.nodes[element[0]];
            let n2 = mesh.nodes[element[1]];
            let n3 = mesh.nodes[element[2]];
            let n4 = mesh.nodes[element[3]];

            // Placeholder for element stiffness matrix (Ke).
            // For a real implementation, this would be derived from material properties and element geometry.
            let ke = self.assemble_tetrahedron_stiffness_matrix(n1, n2, n3, n4, material)?;

            // Assemble Ke into K_global and Fe into F_global.
            // This is a simplified assembly process.
            for i in 0..4 {
                for j in 0..4 {
                    for dof_i in 0..dof_per_node {
                        for dof_j in 0..dof_per_node {
                            // Ensure node indices are within bounds.
            if element[i] >= num_nodes || element[j] >= num_nodes {
                println!("DEBUG: Element {:?} contains out-of-bounds node index. num_nodes: {}", element, num_nodes);
                return Err(EngineError::SolverFailed(format!("Element {} contains out-of-bounds node index.", elem_idx)));
            }
            let global_row = element[i] * dof_per_node + dof_i;
            let global_col = element[j] * dof_per_node + dof_j;
            k_global[(global_row, global_col)] += ke[(i * dof_per_node + dof_i, j * dof_per_node + dof_j)];
                        }
                    }
                }
            }
        }

        // 3. Apply boundary conditions.
        let mut prescribed_dofs = Vec::new();
        let mut prescribed_values = Vec::new();

        for bc in &problem.physics.boundary_conditions {
            // Find nodes belonging to the specified region.
            if let Some(region_nodes_indices) = problem.mesh.as_ref().unwrap().boundary_regions.get(&bc.region) {
                for &node_idx in region_nodes_indices {
                    match bc.condition_type.as_str() {
                        "Dirichlet" => {
                            // Apply displacement boundary conditions.
                            for i in 0..dof_per_node {
                                if bc.value[i].is_finite() { // Only apply if value is not NaN (meaning unconstrained)
                                    prescribed_dofs.push(node_idx * dof_per_node + i);
                                    prescribed_values.push(bc.value[i]);
                                }
                            }
                        },
                        "Force" => {
                            // Apply nodal forces.
                            for i in 0..dof_per_node {
                                f_global[node_idx * dof_per_node + i] += bc.value[i];
                            }
                        },
                        _ => return Err(EngineError::SolverFailed(format!("Unsupported boundary condition type: {}", bc.condition_type))),
                    }
                }
            }
        }

        // Modify K_global and F_global for prescribed DOFs.
        for (&dof_idx, &value) in prescribed_dofs.iter().zip(prescribed_values.iter()) {
            // Set row and column to zero, then set diagonal to 1 and force to prescribed value.
            for col in 0..total_dof {
                k_global[(dof_idx, col)] = 0.0;
            }
            for row in 0..total_dof {
                k_global[(row, dof_idx)] = 0.0;
            }
            k_global[(dof_idx, dof_idx)] = 1.0;
            f_global[dof_idx] = value;
        }

        // 4. Solve for displacements (U).
        let u_global = k_global.try_inverse().ok_or_else(|| EngineError::SolverFailed("Global stiffness matrix is singular.".to_string()))? * f_global;

        // 5. Return displacements as solution data.
        println!("--- FemSolver Finished ---");

        Ok(super::SolverSolutionData {
            data: u_global.iter().cloned().collect(), // Convert DVector to Vec<f64>
        })
    }
}

impl FemSolver {
    /// Placeholder for assembling the element stiffness matrix for a tetrahedron.
    /// This is a highly simplified version and needs proper implementation.
    fn assemble_tetrahedron_stiffness_matrix(
        &self,
        _n1: [f64; 3],
        _n2: [f64; 3],
        _n3: [f64; 3],
        _n4: [f64; 3],
        material: &Material,
    ) -> Result<DMatrix<f64>, EngineError> {
        // For a real FEM solver, this would involve:
        // 1. Calculating the Jacobian and inverse Jacobian.
        // 2. Forming the B matrix (strain-displacement matrix).
        // 3. Forming the D matrix (constitutive matrix from material properties).
        // 4. Integrating B^T * D * B over the element volume.

        // For now, return a dummy 12x12 matrix (4 nodes * 3 DOF/node).
        // This will allow the code to compile and the overall structure to be tested.
        let youngs_modulus = material.youngs_modulus;
        let _poissons_ratio = material.poissons_ratio;

        // A very simplified placeholder for a stiffness matrix.
        // This does NOT represent a correct physical stiffness matrix.
        let mut ke = DMatrix::<f64>::zeros(12, 12);
        ke[(0,0)] = youngs_modulus; // Just to make it non-zero

        Ok(ke)
    }

    /// Calculates the volume of each tetrahedron in the mesh.
    /// This function is kept for now but will be replaced by actual FEM results.
    #[allow(dead_code)]
    fn calculate_tetrahedron_volumes(&self, mesh: &Mesh) -> Result<Vec<f64>, EngineError> {
        let mut volumes = Vec::with_capacity(mesh.elements.len());

        for element in &mesh.elements {
            if element.len() != 4 {
                return Err(EngineError::SolverFailed("Invalid tetrahedron element found with node count != 4".to_string()));
            }

            // Get the coordinates of the 4 nodes of the tetrahedron.
            let p1 = mesh.nodes[element[0]];
            let p2 = mesh.nodes[element[1]];
            let p3 = mesh.nodes[element[2]];
            let p4 = mesh.nodes[element[3]];

            // Calculate the volume of the tetrahedron using the formula:
            // V = |(a-d) . ((b-d) x (c-d))| / 6
            // where a, b, c, d are the coordinates of the vertices.
            let v1 = (p1[0] - p4[0], p1[1] - p4[1], p1[2] - p4[2]);
            let v2 = (p2[0] - p4[0], p2[1] - p4[1], p2[2] - p4[2]);
            let v3 = (p3[0] - p4[0], p3[1] - p4[1], p3[2] - p4[2]);

            let cross_product = (
                v2.1 * v3.2 - v2.2 * v3.1,
                v2.2 * v3.0 - v2.0 * v3.2,
                v2.0 * v3.1 - v2.1 * v3.0,
            );

            let dot_product = v1.0 * cross_product.0 + v1.1 * cross_product.1 + v1.2 * cross_product.2;
            let volume = dot_product.abs() / 6.0;

            volumes.push(volume);
        }

        Ok(volumes)
    }
}