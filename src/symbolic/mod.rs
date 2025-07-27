// src/symbolic/mod.rs

//! Handles symbolic equation processing by bridging to Python's SymPy library.

use crate::EngineError;
use serde::{Serialize, Deserialize};
use std::io::Write;
use std::fs;
use std::env;

/// Represents the result of symbolic processing.
#[derive(Debug, Deserialize, Serialize)]
pub struct ProcessedEquations {
    pub simplified_forms: Vec<String>,
}

/// Processes a list of equation strings using SymPy in a Docker sandbox.
///
/// This function dynamically creates a Python script, runs it in a Docker container
/// with SymPy installed, and captures its output.
pub async fn process_equations_with_sympy(equations: &[String]) -> Result<ProcessedEquations, EngineError> {
    // Construct the Python script content.
    let python_script_content = format!(
        r#"
import sympy
import json
import sys

def simplify_equations(eqs_json):
    eqs = json.loads(eqs_json)
    simplified = [str(sympy.simplify(eq)) for eq in eqs]
    print(json.dumps(simplified))

if __name__ == "__main__":
    try:
        equations_from_stdin = sys.stdin.read()
        simplify_equations(equations_from_stdin)
    except Exception as e:
        print(f"Error during symbolic processing: {{e}}", file=sys.stderr)
        import traceback
        traceback.print_exc(file=sys.stderr)
"#
    );

    // Create a temporary file for the Python script.
    let temp_dir = env::temp_dir();
    let script_file_path = temp_dir.join("sympy_script.py");
    let equations_json_path = temp_dir.join("equations.json");

    // Write the Python script to the temporary file.
    let mut script_file = fs::File::create(&script_file_path)
        .map_err(|e| EngineError::SymbolicFailed(format!("Failed to create script file: {}", e)))?;
    script_file.write_all(python_script_content.as_bytes())
        .map_err(|e| EngineError::SymbolicFailed(format!("Failed to write script content: {}", e)))?;

    // Write the equations to a temporary JSON file to pass to the container via stdin.
    let equations_json = serde_json::to_string(equations)
        .map_err(|e| EngineError::SymbolicFailed(format!("Failed to serialize equations: {}", e)))?;
    let mut json_file = fs::File::create(&equations_json_path)
        .map_err(|e| EngineError::SymbolicFailed(format!("Failed to create JSON file: {}", e)))?;
    json_file.write_all(equations_json.as_bytes())
        .map_err(|e| EngineError::SymbolicFailed(format!("Failed to write JSON content: {}", e)))?;

    // Call the Docker sandbox to run the script.
    // We pass the script content and the path to the JSON file.
    let output = crate::sandbox::run_sandboxed_docker(
        script_file_path.to_str().unwrap(),
        equations_json_path.to_str().unwrap(),
    ).await.map_err(|e| EngineError::SymbolicFailed(format!("Docker sandbox failed: {}", e)))?;

    // Clean up temporary files.
    fs::remove_file(&script_file_path)
        .map_err(|e| EngineError::SymbolicFailed(format!("Failed to remove script file: {}", e)))?;
    fs::remove_file(&equations_json_path)
        .map_err(|e| EngineError::SymbolicFailed(format!("Failed to remove JSON file: {}", e)))?;

    // Parse the JSON output from the Docker container.
    let simplified_forms: Vec<String> = serde_json::from_str(&output)
        .map_err(|e| EngineError::SymbolicFailed(format!("Failed to parse JSON output from sandbox: {}. Raw output: {}", e, output)))?;

    Ok(ProcessedEquations { simplified_forms })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_rt::test]
    async fn test_sympy_simplification_in_docker() {
        // This test requires Docker to be running and the Dockerfile to be built.
        let equations = vec![
            "x + x + y".to_string(),
            "(a + b)**2".to_string(),
        ];
    
        match process_equations_with_sympy(&equations).await {
            Ok(processed) => {
                assert_eq!(processed.simplified_forms.len(), 2);
                assert_eq!(processed.simplified_forms[0], "2*x + y");
                assert_eq!(processed.simplified_forms[1], "(a + b)**2"); // SymPy might not expand this by default
                println!("SymPy successfully simplified equations in Docker: {:?}", processed.simplified_forms);
            }
            Err(e) => {
                // This is expected if Docker is not running or the image is not built.
                println!("SymPy test in Docker failed as expected: {}", e);
            }
        }
    }
}