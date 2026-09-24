fn test_async() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async { println!("Async works!"); });
}
