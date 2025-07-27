pub mod kernel;
pub mod meshing;
pub mod symbolic;
pub mod solver;
pub mod sandbox;
pub mod provenance;

// Re-exporting core numerical types for easier access by other modules.
pub use kernel::{Matrix, Vector};

// --- Return Types and Errors ---

#[derive(Debug, serde::Serialize)]
pub struct Solution {
    pub id: String,
    pub mesh: Mesh,
    pub processed_equations: Option<symbolic::ProcessedEquations>,
    pub data: Vec<f64>, // Raw solution data
    pub provenance_chain: Vec<provenance::ProvenanceRecord>,
}

#[derive(Debug)]
pub enum EngineError {
    MeshingFailed(String),
    SymbolicFailed(String),
    SolverFailed(String),
    PluginNotFound(String),
    ProvenanceFailed(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            EngineError::MeshingFailed(s) => write!(f, "Meshing failed: {}", s),
            EngineError::SymbolicFailed(s) => write!(f, "Symbolic processing failed: {}", s),
            EngineError::SolverFailed(s) => write!(f, "Solver failed: {}", s),
            EngineError::PluginNotFound(s) => write!(f, "Plugin not found: {}", s),
            EngineError::ProvenanceFailed(s) => write!(f, "Provenance failed: {}", s),
        }
    }
}

impl std::error::Error for EngineError {}

// --- Solver Manager ---

struct SolverManager {
    solvers: Vec<Box<dyn solver::Solver>>,
}

impl SolverManager {
    fn new() -> Self {
        SolverManager {
            solvers: vec![Box::new(solver::DummySolver), Box::new(solver::fem_solver::FemSolver), Box::new(solver::fdm_solver::FdmSolver)],
        }
    }

    fn get_solver(&self, name: &str) -> Result<&dyn solver::Solver, EngineError> {
        self.solvers
            .iter()
            .find(|s| s.name() == name)
            .map(|s| s.as_ref())
            .ok_or_else(|| EngineError::PluginNotFound(name.to_string()))
    }
}

// --- Core Engine Facade ---

pub struct CoreEngine {
    solver_manager: SolverManager,
    provenance_chain: provenance::ProvenanceChain,
}

impl CoreEngine {
    pub fn new() -> Self {
        CoreEngine {
            solver_manager: SolverManager::new(),
            provenance_chain: provenance::ProvenanceChain::new(),
        }
    }

    /// The main entry point for running a simulation.
    pub async fn run_simulation(&mut self, mut problem: ProblemDefinition) -> Result<Solution, EngineError> {
        println!("Received simulation task: {}", problem.id);

        // Record initial problem definition
        let problem_json = serde_json::to_string(&problem).map_err(|e| EngineError::ProvenanceFailed(e.to_string()))?;
        self.provenance_chain.add_record(
            "problem_definition".to_string(),
            problem_json.as_bytes(),
            env!("CARGO_PKG_VERSION").to_string(),
            serde_json::json!({"problem_id": problem.id}),
        ).map_err(|e| EngineError::ProvenanceFailed(e.to_string()))?;

        // 1. Generate mesh from geometry
        let mesh = self.generate_mesh(&problem.geometry)?;
        problem.mesh = Some(mesh);
        let mesh_json = serde_json::to_string(&problem.mesh).map_err(|e| EngineError::ProvenanceFailed(e.to_string()))?;
        self.provenance_chain.add_record(
            "mesh_generation".to_string(),
            mesh_json.as_bytes(),
            env!("CARGO_PKG_VERSION").to_string(),
            serde_json::json!({"geometry_type": format!("{:?}", problem.geometry)}),
        ).map_err(|e| EngineError::ProvenanceFailed(e.to_string()))?;

        // 2. Process physics equations (symbolic engine)
        if !problem.physics.equations.is_empty() {
            let processed_equations = self.process_equations(&problem.physics.equations).await?;
            problem.physics.processed_equations = Some(processed_equations);
            let processed_equations_json = serde_json::to_string(&problem.physics.processed_equations).map_err(|e| EngineError::ProvenanceFailed(e.to_string()))?;
            self.provenance_chain.add_record(
                "symbolic_processing".to_string(),
                processed_equations_json.as_bytes(),
                env!("CARGO_PKG_VERSION").to_string(),
                serde_json::json!({"equations": problem.physics.equations}),
            ).map_err(|e| EngineError::ProvenanceFailed(e.to_string()))?;
        }

        // 3. Select and run solver
        let solver = self.solver_manager.get_solver(&problem.solver_settings.solver_name)?;
        let solution_data = solver.solve(&mut problem)?;
        let solution_data_json = serde_json::to_string(&solution_data).map_err(|e| EngineError::ProvenanceFailed(e.to_string()))?;
        self.provenance_chain.add_record(
            "solver_run".to_string(),
            solution_data_json.as_bytes(),
            env!("CARGO_PKG_VERSION").to_string(),
            serde_json::json!({"solver_name": problem.solver_settings.solver_name}),
        ).map_err(|e| EngineError::ProvenanceFailed(e.to_string()))?;

        // Return solution
        Ok(Solution {
            id: problem.id.clone(),
            mesh: problem.mesh.take().unwrap(),
            processed_equations: problem.physics.processed_equations.take(),
            data: solution_data.data,
            provenance_chain: self.provenance_chain.drain_records(),
        })
    }

    /// Generates a mesh from a given geometry definition.
    pub fn generate_mesh(&mut self, geo_def: &GeometryDefinition) -> Result<Mesh, EngineError> {
        meshing::generate_mesh_from_geo(geo_def)
    }

    /// Processes physics equations using the symbolic engine.
    pub async fn process_equations(&mut self, equations: &[String]) -> Result<symbolic::ProcessedEquations, EngineError> {
        symbolic::process_equations_with_sympy(equations)
            .await
            .map_err(|e| EngineError::SymbolicFailed(e.to_string()))
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ProblemDefinition {
    pub id: String,
    pub geometry: GeometryDefinition,
    pub physics: PhysicsDefinition,
    pub solver_settings: SolverSettings,
    pub mesh: Option<Mesh>,
}

/// Defines the geometry for the simulation.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum GeometryDefinition {
    File(String), // Path to a CAD file (e.g., STEP, IGES)
    Primitive(GeometricPrimitive), // A basic, built-in shape
}

/// Describes a simple geometric primitive.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GeometricPrimitive {
    pub shape: String, // e.g., "cube", "sphere"
    pub dimensions: Vec<f64>,
}

/// Contains the physical equations and boundary conditions.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct PhysicsDefinition {
    pub equations: Vec<String>, // e.g., "div(grad(T)) = 0"
    pub boundary_conditions: Vec<BoundaryCondition>,
    pub material: Material,
    pub processed_equations: Option<symbolic::ProcessedEquations>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BoundaryCondition {
    pub region: String, // Name of the geometric region
    pub condition_type: String, // e.g., "Dirichlet", "Neumann", "Force"
    pub value: Vec<f64>, // For Dirichlet: [ux, uy, uz], For Force: [fx, fy, fz]
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Material {
    pub youngs_modulus: f64,
    pub poissons_ratio: f64,
}

/// Specifies which solver to use and its parameters.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SolverSettings {
    pub solver_name: String, // e.g., "FEM_LinearStatic", "PINN_FluidFlow"
    pub tolerance: f64,
    pub max_iterations: u32,
}

/// Represents a discretized simulation domain (the mesh).
/// This is a key data structure passed between the meshing engine and solvers.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Mesh {
    pub nodes: Vec<[f64; 3]>,
    pub elements: Vec<Vec<usize>>,
    pub element_type: String, // e.g., "Tetrahedron", "Hexahedron"
    pub boundary_regions: std::collections::HashMap<String, Vec<usize>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_rt::test]
    async fn test_e2e_simulation_run_with_dummy_solver() {
        let mut engine = CoreEngine::new();

        let problem = ProblemDefinition {
            id: "e2e_test_dummy_01".to_string(),
            geometry: GeometryDefinition::Primitive(GeometricPrimitive {
                shape: "cube".to_string(),
                dimensions: vec![1.0, 1.0, 1.0],
            }),
            physics: PhysicsDefinition {
                equations: vec!["2*x=y".to_string()],
                boundary_conditions: vec![],
                material: Material {
                    youngs_modulus: 1.0,
                    poissons_ratio: 0.0,
                },
                processed_equations: None,
            },
            solver_settings: SolverSettings {
                solver_name: "DummySolver".to_string(),
                tolerance: 1e-5,
                max_iterations: 10,
            },
            mesh: None,
        };

        match engine.run_simulation(problem).await {
            Ok(solution) => {
                println!("Simulation for {} completed successfully.", solution.id);
                assert_eq!(solution.data.len(), solution.mesh.nodes.len());
            },
            Err(e) => {
                // This test should not fail if the environment is set up correctly.
                panic!("E2E simulation failed unexpectedly: {}", e);
            }
        }
    }

    #[actix_rt::test]
    async fn test_e2e_simulation_run_with_fem_solver() {
        let mut engine = CoreEngine::new();

        let problem = ProblemDefinition {
            id: "e2e_test_fem_01".to_string(),
            geometry: GeometryDefinition::Primitive(GeometricPrimitive {
                shape: "cube".to_string(),
                dimensions: vec![1.0, 1.0, 1.0],
            }),
            physics: PhysicsDefinition {
                equations: vec![],
                boundary_conditions: vec![
                    BoundaryCondition {
                        region: "face_x_neg".to_string(), // Assuming Gmsh names faces
                        condition_type: "Dirichlet".to_string(),
                        value: vec![0.0, 0.0, 0.0],
                    },
                    BoundaryCondition {
                        region: "face_x_pos".to_string(),
                        condition_type: "Force".to_string(),
                        value: vec![100.0, 0.0, 0.0],
                    },
                ],
                material: Material {
                    youngs_modulus: 200e9, // Steel
                    poissons_ratio: 0.3,
                },
                processed_equations: None,
            },
            solver_settings: SolverSettings {
                solver_name: "FemSolver".to_string(),
                tolerance: 1e-5,
                max_iterations: 10,
            },
            mesh: None,
        };

        match engine.run_simulation(problem).await {
            Ok(solution) => {
                println!("FEM Simulation for {} completed successfully.", solution.id);
                assert_eq!(solution.data.len(), solution.mesh.elements.len());
                // Check that the sum of the element volumes is approximately the volume of the cube (1.0)
                let total_volume: f64 = solution.data.iter().sum();
                println!("Total calculated volume: {}", total_volume);
                assert!((total_volume - 1.0).abs() < 1e-9, "Total volume should be close to 1.0");
            },
            Err(e) => {
                println!("E2E FEM simulation failed!");
                panic!("E2E FEM simulation failed unexpectedly: {}", e);
            }
        }
    }

    #[actix_rt::test]
    async fn test_e2e_simulation_run_with_fdm_solver() {
        let mut engine = CoreEngine::new();

        let problem = ProblemDefinition {
            id: "e2e_test_fdm_01".to_string(),
            geometry: GeometryDefinition::Primitive(GeometricPrimitive {
                shape: "cube".to_string(), // Geometry is not directly used by FDM, but required
                dimensions: vec![1.0, 1.0, 1.0],
            }),
            physics: PhysicsDefinition {
                equations: vec![],
                boundary_conditions: vec![],
                material: Material {
                    youngs_modulus: 1.0,
                    poissons_ratio: 0.0,
                },
                processed_equations: None,
            },
            solver_settings: SolverSettings {
                solver_name: "FdmSolver".to_string(),
                tolerance: 1e-5,
                max_iterations: 10,
            },
            mesh: None,
        };

        match engine.run_simulation(problem).await {
            Ok(solution) => {
                println!("FDM Simulation for {} completed successfully.", solution.id);
                // For 1D heat conduction, we expect a linear temperature profile.
                // T(x) = T_left - (T_left - T_right) * x / L
                // With T_left = 100, T_right = 0, L = 1, and 11 nodes (0 to 10).
                // Node i is at x = i * dx = i * (1/10).
                // T(i) = 100 - (100 - 0) * (i/10) = 100 - 10 * i
                let expected_temperatures: Vec<f64> = (0..11).map(|i| 100.0 - 10.0 * i as f64).collect();

                assert_eq!(solution.data.len(), expected_temperatures.len());
                for (i, &val) in solution.data.iter().enumerate() {
                    assert!((val - expected_temperatures[i]).abs() < 1e-9, "Node {} temperature mismatch: expected {}, got {}", i, expected_temperatures[i], val);
                }
            },
            Err(e) => {
                panic!("E2E FDM simulation failed unexpectedly: {}", e);
            }
        }
    }
}