
// src/sandbox/mod.rs

//! Provides sandboxed execution environments for user code.

use wasmer::{Store, Module, Instance, Function, Value};

/// Executes a WebAssembly (Wasm) module in a sandboxed environment.
///
/// This function uses the Wasmer runtime to compile and run a Wasm module.
/// The Wasm module is completely isolated from the host system, with no access
/// to the filesystem, network, or other resources unless explicitly granted.
///
/// # Arguments
/// * `wasm_bytes` - A slice of bytes representing the Wasm module.
///
/// # Returns
/// A `Result` containing the integer result from the Wasm module's exported
/// `run` function, or an error string.
pub fn run_sandboxed_wasm(wasm_bytes: &[u8]) -> Result<i32, String> {
    // 1. Create a new Wasmer Store. The Store holds all the runtime state.
    let mut store = Store::default();

    // 2. Compile the Wasm bytes into a Module.
    // This is a platform-independent representation of the compiled code.
    let module = Module::new(&store, wasm_bytes)
        .map_err(|e| format!("Failed to compile Wasm module: {}", e))?;

    // 3. Create an import object. Since our guest module doesn't import any
    // functions from the host, this is empty.
    let import_object = wasmer::imports! {};

    // 4. Instantiate the module.
    // This creates an `Instance`, which is a ready-to-run Wasm module.
    // The instance is sandboxed within the Store.
    let instance = Instance::new(&mut store, &module, &import_object)
        .map_err(|e| format!("Failed to instantiate Wasm module: {}", e))?;

    // 5. Get the exported `run` function from the Wasm instance.
    let run_func: &Function = instance.exports.get_function("run")
        .map_err(|e| format!("Failed to find exported 'run' function: {}", e))?;

    // 6. Call the exported function with some arguments.
    let result = run_func.call(&mut store, &[Value::I32(5), Value::I32(10)])
        .map_err(|e| format!("Failed to call 'run' function: {}", e))?;

    // 7. Get the result from the function call.
    result[0].i32().ok_or_else(|| "Wasm function did not return an i32 value".to_string())
}

pub async fn run_sandboxed_docker(_script_path: &str, _script_content: &str) -> Result<String, String> {
    use docker_api::Docker;
    
    use docker_api::opts::{ImageBuildOpts, ContainerCreateOpts, LogsOpts, ContainerRemoveOpts};
    use futures_util::stream::StreamExt;

    // 1. Create a new Docker instance.
    let docker = Docker::new("unix:///var/run/docker.sock").unwrap();

    // 2. Build the Docker image.
    let images = docker.images();
    let build_opts = ImageBuildOpts::builder(".").dockerfile("Dockerfile").build();
    let mut stream = images.build(&build_opts);
    while let Some(result) = stream.next().await {
        result.unwrap();
    }

    // 3. Create the container.
    let create_opts = ContainerCreateOpts::builder().image("python:3.10-slim").build();
    let container = docker.containers().create(&create_opts).await.unwrap();

    // 4. Start the container.
    container.start().await.unwrap();

    // 5. Wait for the container to finish and get the logs.
    container.wait().await.unwrap();
    let logs_stream = container.logs(&LogsOpts::builder().stdout(true).stderr(true).build());
    let logs: Vec<String> = logs_stream.map(|l| {
        match l.unwrap() {
            docker_api::conn::TtyChunk::StdOut(bytes) => String::from_utf8(bytes).unwrap(),
            docker_api::conn::TtyChunk::StdErr(bytes) => String::from_utf8(bytes).unwrap(),
            _ => "".to_string(),
        }
    }).collect().await;

    // 6. Clean up the container.
    container.remove(&ContainerRemoveOpts::builder().force(true).build()).await.unwrap();

    Ok(logs.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    

    // A simple Wasm module written in WAT (WebAssembly Text Format) for testing.
    // It exports a single function `run` that takes two i32 numbers and returns their sum.
    const GUEST_WAT: &str = r#"
    (module
        (func $add (param $a i32) (param $b i32) (result i32)
            local.get $a
            local.get $b
            i32.add)
        (export "run" (func $add)))
    "#;

    #[test]
    fn test_wasm_sandboxing() {
        // Use the wasmer CLI to compile our WAT to Wasm bytes.
        // This requires `wasmer` to be installed and in the PATH.
        let wasm_bytes = wasmer::wat2wasm(GUEST_WAT.as_bytes())
            .expect("Failed to compile WAT to Wasm. Is the `wasmer` CLI installed?");

        // Run the compiled Wasm bytes in our sandbox.
        match run_sandboxed_wasm(&wasm_bytes) {
            Ok(result) => {
                // The guest module should add 5 + 10 = 15.
                assert_eq!(result, 15);
                println!("Wasm sandbox test successful! Result: {}", result);
            }
            Err(e) => {
                panic!("Wasm sandbox test failed: {}", e);
            }
        }
    }

    // #[actix_rt::test]
    // async fn test_docker_sandboxing() {
    //     // This test requires Docker to be running.
    //     let script_content = "print(1 + 2)";
    //     let script_path = "script.py";
    // 
    //     // Write the script to a file.
    //     std::fs::write(script_path, script_content).unwrap();
    // 
    //     match run_sandboxed_docker(script_path, script_content).await {
    //         Ok(output) => {
    //             assert_eq!(output.trim(), "3");
    //             println!("Docker sandbox test successful! Output: {}", output);
    //         }
    //         Err(e) => {
    //             panic!("Docker sandbox test failed: {}", e);
    //         }
    //     }
    // 
    //     // Clean up the script file.
    //     std::fs::remove_file(script_path).unwrap();
    // }
}
