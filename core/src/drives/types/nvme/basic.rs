use crate::ui::progress::ProgressBar;
use anyhow::{anyhow, Result};
use std::process::Command;
use std::thread;
use std::time::Duration;

pub struct NVMeWipe;

impl NVMeWipe {
    pub fn secure_erase(device_path: &str) -> Result<()> {
        println!("Starting NVMe secure erase on {}", device_path);

        let _device_info = Self::get_nvme_info(device_path)?;

        if Self::supports_format_nvm(device_path)? {
            let mut cmd = Command::new("nvme");
            cmd.args(["format", device_path, "--ses=1", "--force"]);
            Self::run_command_with_bar(&mut cmd, "Format NVM secure erase")
        } else if Self::supports_sanitize(device_path)? {
            let mut cmd = Command::new("nvme");
            cmd.args(["sanitize", device_path, "--crypto-erase", "--force"]);
            Self::run_command_with_bar(&mut cmd, "Sanitize crypto erase")
        } else {
            Err(anyhow!(
                "No secure erase method available for this NVMe device"
            ))
        }
    }

    fn run_command_with_bar(cmd: &mut Command, label: &str) -> Result<()> {
        println!("Using {}...", label);
        let mut process = cmd.spawn()?;
        let mut bar = ProgressBar::new(48);

        loop {
            match process.try_wait()? {
                Some(status) => {
                    bar.render(100.0, None, None);
                    if status.success() {
                        println!("\n{} completed successfully", label);
                        return Ok(());
                    } else {
                        return Err(anyhow!("{} failed", label));
                    }
                }
                None => {
                    bar.render(50.0, None, None);
                    thread::sleep(Duration::from_millis(200));
                }
            }
        }
    }

    fn get_nvme_info(device_path: &str) -> Result<String> {
        let output = Command::new("nvme")
            .args(["id-ctrl", device_path])
            .output()?;
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to get NVMe info: {}", error));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn supports_format_nvm(device_path: &str) -> Result<bool> {
        Ok(Self::get_nvme_info(device_path)?.contains("Format NVM Supported"))
    }

    fn supports_sanitize(device_path: &str) -> Result<bool> {
        Ok(Self::get_nvme_info(device_path)?.contains("Sanitize Operation Supported"))
    }
}
