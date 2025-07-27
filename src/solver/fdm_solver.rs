
// src/solver/fdm_solver.rs

//! A basic Finite Difference Method (FDM) solver.

use crate::{ProblemDefinition, EngineError};
use crate::solver::Solver;
use nalgebra::{DMatrix, DVector};

/// A simple FDM solver for 1D steady-state heat conduction.
///
/// This solver discretizes a 1D domain and solves for the temperature
/// distribution given boundary conditions.
pub struct FdmSolver;

impl Solver for FdmSolver {
    fn name(&self) -> &'static str {
        "FdmSolver"
    }

    fn solve(&self, _problem: &mut ProblemDefinition) -> Result<super::SolverSolutionData, EngineError> {
        println!("--- Running FdmSolver (1D Heat Conduction) ---");

        // For simplicity, we'll assume a 1D domain of length L with N nodes.
        // The problem definition should ideally contain these parameters.
        // For now, we hardcode them for demonstration.
        let length = 1.0; // Length of the 1D domain
        let num_nodes = 11; // Number of nodes (including boundary nodes)
        let _dx = length / (num_nodes - 1) as f64; // Grid spacing

        // Initialize global stiffness matrix (A) and load vector (B).
        // For 1D steady-state heat conduction (d^2T/dx^2 = 0),
        // the discretized equation is (T_i-1 - 2*T_i + T_i+1) / dx^2 = 0
        // which simplifies to T_i-1 - 2*T_i + T_i+1 = 0
        let mut a_global = DMatrix::<f64>::zeros(num_nodes, num_nodes);
        let mut b_global = DVector::<f64>::zeros(num_nodes);

        // Assemble the system (internal nodes).
        for i in 1..num_nodes - 1 {
            a_global[(i, i - 1)] = 1.0;
            a_global[(i, i)] = -2.0;
            a_global[(i, i + 1)] = 1.0;
        }

        // Apply boundary conditions.
        // We'll assume fixed temperatures at both ends.
        // T(0) = T_left, T(L) = T_right
        let t_left = 100.0;
        let t_right = 0.0;

        // Node 0 (left boundary)
        a_global[(0, 0)] = 1.0;
        b_global[0] = t_left;

        // Node N-1 (right boundary)
        a_global[(num_nodes - 1, num_nodes - 1)] = 1.0;
        b_global[num_nodes - 1] = t_right;

        // Solve for nodal temperatures (T).
        let t_solution = a_global.try_inverse().ok_or_else(|| EngineError::SolverFailed("FDM matrix is singular.".to_string()))? * b_global;

        // Return temperatures as solution data.
        println!("--- FdmSolver Finished ---");

        Ok(super::SolverSolutionData {
            data: t_solution.iter().cloned().collect(), // Convert DVector to Vec<f64>
        })
    }
}
