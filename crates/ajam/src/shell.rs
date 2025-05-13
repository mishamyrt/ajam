use tokio::process::Command;


pub(crate) async fn run_command(command: &str, args: &Vec<String>) -> Result<String, String> {
    let output = Command::new(command).args(args).output().await.expect("failed to run command");
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
