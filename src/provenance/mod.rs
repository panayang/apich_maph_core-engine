// src/provenance/mod.rs

//! Implements the V&V / Provenance Engine for tracking simulation data lineage.

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};

/// Represents a single record in the provenance chain.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub data_hash: String,
    pub software_version: String,
    pub previous_record_hash: Option<String>,
    pub metadata: serde_json::Value,
}

impl ProvenanceRecord {
    /// Creates a new ProvenanceRecord.
    pub fn new(
        event_type: String,
        data: &[u8],
        software_version: String,
        previous_record_hash: Option<String>,
        metadata: serde_json::Value,
    ) -> Self {
        let timestamp = Utc::now();
        let data_hash = calculate_hash(data);

        ProvenanceRecord {
            timestamp,
            event_type,
            data_hash,
            software_version,
            previous_record_hash,
            metadata,
        }
    }

    /// Calculates the hash of the current record for linking.
    pub fn calculate_record_hash(&self) -> String {
        let serialized = serde_json::to_string(self).expect("Failed to serialize ProvenanceRecord");
        calculate_hash(serialized.as_bytes())
    }
}

/// Calculates the SHA256 hash of a byte slice.
fn calculate_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Manages the chain of ProvenanceRecords.
pub struct ProvenanceChain {
    records: Vec<ProvenanceRecord>,
}

impl ProvenanceChain {
    /// Creates a new empty ProvenanceChain.
    pub fn new() -> Self {
        ProvenanceChain { records: Vec::new() }
    }

    /// Adds a new record to the chain.
    pub fn add_record(
        &mut self,
        event_type: String,
        data: &[u8],
        software_version: String,
        metadata: serde_json::Value,
    ) -> Result<(), String> {
        let previous_record_hash = self.records.last().map(|r| r.calculate_record_hash());
        let record = ProvenanceRecord::new(
            event_type,
            data,
            software_version,
            previous_record_hash,
            metadata,
        );
        self.records.push(record);
        Ok(())
    }

    /// Returns a reference to the records in the chain.
    pub fn records(&self) -> &[ProvenanceRecord] {
        &self.records
    }

    /// Serializes the entire chain to a JSON string.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self.records)
            .map_err(|e| format!("Failed to serialize provenance chain: {}", e))
    }

    /// Deserializes a provenance chain from a JSON string.
    pub fn from_json(json_str: &str) -> Result<Self, String> {
        let records = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to deserialize provenance chain: {}", e))?;
        Ok(ProvenanceChain { records })
    }

    /// Consumes the ProvenanceChain and returns its records.
    pub fn take_records(self) -> Vec<ProvenanceRecord> {
        self.records
    }

    /// Drains all records from the ProvenanceChain, leaving it empty.
    pub fn drain_records(&mut self) -> Vec<ProvenanceRecord> {
        std::mem::take(&mut self.records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provenance_record_creation() {
        let data = b"some simulation data";
        let record = ProvenanceRecord::new(
            "mesh_generation".to_string(),
            data,
            "v1.0.0".to_string(),
            None,
            serde_json::json!({"mesh_size": 0.1}),
        );

        assert_eq!(record.event_type, "mesh_generation");
        assert_eq!(record.software_version, "v1.0.0");
        assert!(record.previous_record_hash.is_none());
        assert_eq!(record.metadata["mesh_size"], 0.1);

        let expected_hash = calculate_hash(data);
        assert_eq!(record.data_hash, expected_hash);
    }

    #[test]
    fn test_provenance_chain_linking() {
        let mut chain = ProvenanceChain::new();

        let data1 = b"initial data";
        chain.add_record(
            "initial_setup".to_string(),
            data1,
            "v1.0.0".to_string(),
            serde_json::json!({"config": "default"}),
        ).unwrap();

        let record1_hash = chain.records()[0].calculate_record_hash();

        let data2 = b"meshed data";
        chain.add_record(
            "mesh_generation".to_string(),
            data2,
            "v1.0.0".to_string(),
            serde_json::json!({"mesh_type": "tetra"}),
        ).unwrap();

        let record2 = &chain.records()[1];
        let record2_hash = record2.calculate_record_hash();
        assert_eq!(record2.event_type, "mesh_generation");
        assert_eq!(record2.previous_record_hash, Some(record1_hash));

        let data3 = b"solver output";
        chain.add_record(
            "solver_run".to_string(),
            data3,
            "v1.0.0".to_string(),
            serde_json::json!({"solver": "fem"}),
        ).unwrap();

        let record3 = &chain.records()[2];
        assert_eq!(record3.event_type, "solver_run");
        assert!(record3.previous_record_hash.is_some());
        assert_eq!(record3.previous_record_hash.as_ref().unwrap().clone(), record2_hash);
    }

    #[test]
    fn test_provenance_chain_serialization() {
        let mut chain = ProvenanceChain::new();

        chain.add_record(
            "initial_setup".to_string(),
            b"data1",
            "v1.0.0".to_string(),
            serde_json::json!({"config": "default"}),
        ).unwrap();

        chain.add_record(
            "mesh_generation".to_string(),
            b"data2",
            "v1.0.0".to_string(),
            serde_json::json!({"mesh_type": "tetra"}),
        ).unwrap();

        let json_output = chain.to_json().unwrap();
        println!("Serialized Provenance Chain:\n{}", json_output);

        let deserialized_chain = ProvenanceChain::from_json(&json_output).unwrap();

        assert_eq!(chain.records().len(), deserialized_chain.records().len());
        assert_eq!(chain.records()[0].event_type, deserialized_chain.records()[0].event_type);
        assert_eq!(chain.records()[1].data_hash, deserialized_chain.records()[1].data_hash);
    }
}
