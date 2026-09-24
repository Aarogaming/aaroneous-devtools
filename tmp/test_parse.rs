fn main() {
    let json = std::fs::read_to_string("d:/aaroneous-devtools/artifacts/local-worker-service/results/FIX-C40-SHADOW-LEARNER-0000.json").unwrap();
    let val: serde_json::Value = serde_json::from_str(&json).unwrap();
    println!("rejection_reason: {:?}", val.get("rejection_reason"));
}
