
pub mod fem_solver;
pub mod fdm_solver;

// src/solver/mod.rs

/// Defines the solver framework, including the `Solver` trait and a dummy solver for testing.

use crate::{ProblemDefinition, EngineError};

/// Represents the raw solution data returned by a solver.
#[derive(Debug, serde::Serialize)]
pub struct SolverSolutionData {
    pub data: Vec<f64>,
}

/// The common interface for all physics solvers.
///
/// A solver is responsible for taking a complete problem definition
/// (including the mesh and processed equations) and computing a solution.
pub trait Solver {
    /// Returns the unique name of the solver.
    fn name(&self) -> &'static str;

    /// Solves the given problem.
    fn solve(&self, problem: &mut ProblemDefinition) -> Result<SolverSolutionData, EngineError>;
}


/// A simple dummy solver for testing the framework.
///
/// This solver does not perform any real calculations. It prints the information
/// it receives and returns a placeholder solution with zeroed data.
pub struct DummySolver;

impl Solver for DummySolver {
    fn name(&self) -> &'static str {
        "DummySolver"
    }

    fn solve(&self, problem: &mut ProblemDefinition) -> Result<SolverSolutionData, EngineError> {
        println!("--- Running DummySolver ---");
        println!("  Problem ID: {}", problem.id);
        println!("  Solver specified: {}", problem.solver_settings.solver_name);

        // In a real solver, we would use the mesh and equations here.
        // For now, we just print some stats.
        if let Some(eqs) = &problem.physics.processed_equations {
            println!("  Number of equations: {}", eqs.simplified_forms.len());
            println!("  Simplified equations: {:?}", eqs.simplified_forms);
        }

        let num_nodes = problem.mesh.as_ref().unwrap().nodes.len();
        println!("  Mesh has {} nodes.", num_nodes);

        // Create a placeholder solution vector of the correct size, filled with zeros.
        let placeholder_data = vec![0.0; num_nodes];

        println!("--- DummySolver Finished ---");

        Ok(SolverSolutionData {
            data: placeholder_data,
        })
    }
}

