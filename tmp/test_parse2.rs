use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Record {
    pub(crate) job_id: String,
    pub(crate) outcome: String,
    pub(crate) input: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) rejection_reason: Option<String>,
}

fn main() {
    let json = std::fs::read_to_string("d:/aaroneous-devtools/artifacts/local-worker-service/results/FIX-C40-SHADOW-LEARNER-0000.json").unwrap();
    let val: Record = serde_json::from_str(&json).unwrap();
    println!("job_id: {}", val.job_id);
    println!("rejection_reason: {:?}", val.rejection_reason);
}
