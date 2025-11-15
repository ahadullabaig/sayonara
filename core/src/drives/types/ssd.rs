use crate::ui::progress::ProgressBar;
use anyhow::{anyhow, Result};
use std::process::Command;
use std::thread;
use std::time::Duration;

pub struct SSDWipe;

impl SSDWipe {
    pub fn secure_erase(device_path: &str) -> Result<()> {
        println!("Attempting hardware secure erase on {}", device_path);

        if !Self::is_secure_erase_supported(device_path)? {
            return Err(anyhow!("Secure erase not supported on this device"));
        }

        Self::unfreeze_drive(device_path)?;
        Self::set_security_password(device_path, "temp123")?;

        let mut cmd = Command::new("hdparm");
        cmd.args([
            "--user-master",
            "u",
            "--security-erase",
            "temp123",
            device_path,
        ]);
        let mut process = cmd.spawn()?;

        let mut bar = ProgressBar::new(48);
        loop {
            match process.try_wait()? {
                Some(status) => {
                    bar.render(100.0, None, None);
                    if status.success() {
                        println!("\nHardware secure erase completed successfully");
                        return Ok(());
                    } else {
                        return Err(anyhow!("Secure erase failed"));
                    }
                }
                None => {
                    bar.render(50.0, None, None);
                    thread::sleep(Duration::from_millis(200));
                }
            }
        }
    }

    fn is_secure_erase_supported(device_path: &str) -> Result<bool> {
        let output = Command::new("hdparm").args(["-I", device_path]).output()?;
        let output_str = String::from_utf8_lossy(&output.stdout);
        Ok(output_str.contains("supported: enhanced erase"))
    }

    fn unfreeze_drive(device_path: &str) -> Result<()> {
        println!("Checking drive freeze status...");
        let output = Command::new("hdparm").args(["-I", device_path]).output()?;
        let output_str = String::from_utf8_lossy(&output.stdout);
        if output_str.contains("frozen") {
            println!("Warning: Drive is frozen. May need power cycle.");
        }
        Ok(())
    }

    fn set_security_password(device_path: &str, password: &str) -> Result<()> {
        let output = Command::new("hdparm")
            .args([
                "--user-master",
                "u",
                "--security-set-pass",
                password,
                device_path,
            ])
            .output()?;
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to set security password: {}", error));
        }
        Ok(())
    }
}
