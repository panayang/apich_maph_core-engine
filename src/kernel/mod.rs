
// src/kernel/mod.rs

//! The numerical kernel of the simulation engine.
//! This module provides fundamental mathematical operations and data structures.

use nalgebra::{DMatrix, DVector};

// Type aliases for clarity throughout the engine.
pub type Matrix = DMatrix<f64>;
pub type Vector = DVector<f64>;

/// A simple function to demonstrate a kernel operation.
pub fn add_vectors(a: &Vector, b: &Vector) -> Option<Vector> {
    if a.len() != b.len() {
        return None;
    }
    Some(a + b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_addition() {
        let a = Vector::from_vec(vec![1.0, 2.0, 3.0]);
        let b = Vector::from_vec(vec![4.0, 5.0, 6.0]);
        let result = add_vectors(&a, &b).unwrap();
        let expected = Vector::from_vec(vec![5.0, 7.0, 9.0]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_vector_addition_mismatched_lengths() {
        let a = Vector::from_vec(vec![1.0, 2.0]);
        let b = Vector::from_vec(vec![4.0, 5.0, 6.0]);
        let result = add_vectors(&a, &b);
        assert!(result.is_none());
    }
}

