use crate::ui::progress::ProgressBar;
use anyhow::Result;
use std::process::Command;
use std::thread;
use std::time::Duration;

pub struct HDDWipe;

impl HDDWipe {
    pub fn secure_erase(device_path: &str) -> Result<()> {
        println!("Starting HDD secure erase on {}", device_path);

        if Self::supports_secure_erase(device_path)? {
            Self::hardware_secure_erase(device_path)
        } else {
            println!("Hardware secure erase not available, use software method instead");
            Ok(())
        }
    }

    fn supports_secure_erase(device_path: &str) -> Result<bool> {
        let output = Command::new("hdparm").args(["-I", device_path]).output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        Ok(output_str.contains("supported: enhanced erase"))
    }

    fn hardware_secure_erase(device_path: &str) -> Result<()> {
        println!("Running ATA secure erase...");

        // set password (blocking)
        let mut bar = ProgressBar::new(48);
        let mut set_cmd = Command::new("hdparm");
        set_cmd.args([
            "--user-master",
            "u",
            "--security-set-pass",
            "temp123",
            device_path,
        ]);
        let _ = set_cmd.spawn()?.wait()?;

        let mut cmd = Command::new("hdparm");
        cmd.args([
            "--user-master",
            "u",
            "--security-erase",
            "temp123",
            device_path,
        ]);
        let mut process = cmd.spawn()?;

        // animate until process ends
        loop {
            match process.try_wait()? {
                Some(status) => {
                    bar.render(100.0, None, None);
                    if status.success() {
                        println!("\nHardware secure erase completed successfully");
                        return Ok(());
                    } else {
                        return Err(anyhow::anyhow!("Hardware secure erase failed"));
                    }
                }
                None => {
                    // show mid-progress with no byte info (animated)
                    bar.render(50.0, None, None);
                    thread::sleep(Duration::from_millis(200));
                }
            }
        }
    }
}
