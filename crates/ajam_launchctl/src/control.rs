use async_trait::async_trait;
use tokio::process::Command;
use std::io;
use crate::agent::LaunchAgent;

#[async_trait]
pub trait LaunchControllable {
    type Error;
    async fn bootstrap(&self) -> Result<(), Self::Error>;
    async fn boot_out(&self) -> Result<(), Self::Error>;
    async fn is_running(&self) -> Result<bool, Self::Error>;
}

fn get_user_id() -> u32 {
    unsafe { libc::geteuid() }
}

fn format_command(agent: &LaunchAgent, command: &str) -> String {
    format!(
        "launchctl {} gui/{} '{}'",
        command,
        get_user_id(),
        agent.path().display()
    )
}

async fn run_shell(command: &str) -> io::Result<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .await?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[async_trait]
impl LaunchControllable for LaunchAgent {
    type Error = io::Error;

    async fn bootstrap(&self) -> Result<(), Self::Error> {
        let cmd = format_command(self, "bootstrap");
        run_shell(&cmd).await.map(|_| ())
    }

    async fn boot_out(&self) -> Result<(), Self::Error> {
        let cmd = format_command(self, "bootout");
        run_shell(&cmd).await.map(|_| ())
    }

    async fn is_running(&self) -> Result<bool, Self::Error> {
        let cmd = format!(
            "launchctl print gui/{}/{}",
            get_user_id(),
            self.label
        );
        match run_shell(&cmd).await {
            Ok(output) => Ok(output.contains("state = running")),
            Err(e) => {
                // Если ошибка, считаем, что агент не запущен
                if e.kind() == io::ErrorKind::Other {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }
}
