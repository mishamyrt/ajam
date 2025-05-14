use serde::{Deserialize, Serialize};
use std::{fs::File, io::Write, path::PathBuf};
use thiserror::Error;

const DEV_NULL: &str = "/dev/null";

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct LaunchAgent {
    pub label: String,
    pub program_arguments: Vec<String>,
    pub standard_out_path: String,
    pub standard_error_path: String,
    pub keep_alive: bool,
    pub run_at_load: bool,
}

#[derive(Error, Debug)]
pub enum LaunchAgentError {
    #[error("Failed to process plist")]
    PListError(#[from] plist::Error),

    #[error("Failed to write plist")]
    WriteError(#[from] std::io::Error),
}

impl LaunchAgent {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            program_arguments: vec![],
            standard_out_path: DEV_NULL.to_string(),
            standard_error_path: DEV_NULL.to_string(),
            keep_alive: false,
            run_at_load: false,
        }
    }

    pub fn exists(label: &str) -> bool {
        let path = Self::path_for(label);
        path.exists()
    }

    pub(crate) fn path_for(label: &str) -> PathBuf {
        let home = std::env::var("HOME").unwrap();
        let file_name = format!("{}.plist", label);
        PathBuf::from(home)
            .join("Library")
            .join("LaunchAgents")
            .join(file_name)
    }

    pub(crate) fn path(&self) -> PathBuf {
        Self::path_for(&self.label)
    }

    fn to_writer<W: Write>(&self, writer: W) -> Result<(), LaunchAgentError> {
        plist::to_writer_xml(writer, self)?;
        Ok(())
    }

    pub fn write(&self) -> Result<(), LaunchAgentError> {
        let path = Self::path_for(&self.label);
        let mut file = File::create(path)?;
        self.to_writer(&mut file)?;
        Ok(())
    }

    pub fn from_file(label: &str) -> Result<Self, LaunchAgentError> {
        let path = Self::path_for(label);

        let agent = plist::from_file(path)?;

        Ok(agent)
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufWriter;

    use super::*;

    #[test]
    fn test_format_plist() {
        let agent = LaunchAgent {
            label: "co.myrt.ajam".to_string(),
            program_arguments: vec![
                "ajam".to_string(),
                "run".to_string(),
            ],
            standard_out_path: DEV_NULL.to_string(),
            standard_error_path: DEV_NULL.to_string(),
            keep_alive: false,
            run_at_load: false,
        };

        let mut buf = BufWriter::new(Vec::new());

        agent.to_writer(&mut buf).unwrap();

        let plist = String::from_utf8(buf.into_inner().unwrap()).unwrap();

        assert!(plist.contains("</dict>"));
        assert!(plist.contains("<key>Label</key>"));
        assert!(plist.contains("<key>ProgramArguments</key>"));
        assert!(plist.contains("<key>StandardOutPath</key>"));
        assert!(plist.contains("<key>StandardErrorPath</key>"));
        assert!(plist.contains("<key>KeepAlive</key>"));
        assert!(plist.contains("<key>RunAtLoad</key>"));

        assert!(plist.contains("co.myrt.ajam"));
    }

    #[test]
    fn test_path() {
        let agent = LaunchAgent {
            label: "co.myrt.ajam".to_string(),
            program_arguments: vec![],
            standard_out_path: DEV_NULL.to_string(),
            standard_error_path: DEV_NULL.to_string(),
            keep_alive: false,
            run_at_load: false,
        };
        let path = PathBuf::from("Library/LaunchAgents/co.myrt.ajam.plist");
        let abs_path = PathBuf::from(std::env::var("HOME").unwrap()).join(path);
        assert_eq!(agent.path(), abs_path);
    }
}
