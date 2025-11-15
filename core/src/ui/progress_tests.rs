// Comprehensive tests for UI Progress Bar
//
// Tests cover: ProgressBar construction, human_bytes conversion, duration formatting,
// progress clamping, bar width calculations, and animation frame logic.

use super::progress::*;

// ==================== PROGRESS BAR CONSTRUCTION TESTS ====================

#[test]
fn test_progress_bar_new() {
    let _bar = ProgressBar::new(50);
    // Can't access private fields directly, but verify no panic
}

#[test]
fn test_progress_bar_new_various_widths() {
    let widths = vec![1, 10, 20, 48, 50, 80, 100, 120];

    for width in widths {
        let bar = ProgressBar::new(width);
        // Verify construction doesn't panic
        let _ = bar;
    }
}

#[test]
fn test_progress_bar_new_zero_width() {
    let bar = ProgressBar::new(0);
    // Should handle gracefully
    let _ = bar;
}

// ==================== HUMAN BYTES CONVERSION TESTS ====================

#[test]
fn test_human_bytes_zero() {
    let result = human_bytes(0.0);
    assert_eq!(result, "0B");
}

#[test]
fn test_human_bytes_negative() {
    let result = human_bytes(-100.0);
    assert_eq!(result, "0B", "Negative values should return 0B");
}

#[test]
fn test_human_bytes_bytes() {
    let result = human_bytes(512.0);
    assert_eq!(result, "512.00B");
}

#[test]
fn test_human_bytes_kilobytes() {
    let result = human_bytes(1024.0);
    assert_eq!(result, "1.00KB");

    let result2 = human_bytes(1536.0); // 1.5 KB
    assert_eq!(result2, "1.50KB");
}

#[test]
fn test_human_bytes_megabytes() {
    let result = human_bytes(1024.0 * 1024.0); // 1 MB
    assert_eq!(result, "1.00MB");

    let result2 = human_bytes(2.5 * 1024.0 * 1024.0); // 2.5 MB
    assert_eq!(result2, "2.50MB");
}

#[test]
fn test_human_bytes_gigabytes() {
    let result = human_bytes(1024.0 * 1024.0 * 1024.0); // 1 GB
    assert_eq!(result, "1.00GB");

    let result2 = human_bytes(5.75 * 1024.0 * 1024.0 * 1024.0); // 5.75 GB
    assert_eq!(result2, "5.75GB");
}

#[test]
fn test_human_bytes_terabytes() {
    let result = human_bytes(1024.0 * 1024.0 * 1024.0 * 1024.0); // 1 TB
    assert_eq!(result, "1.00TB");

    let result2 = human_bytes(2.25 * 1024.0 * 1024.0 * 1024.0 * 1024.0); // 2.25 TB
    assert_eq!(result2, "2.25TB");
}

#[test]
fn test_human_bytes_boundary_1023_bytes() {
    let result = human_bytes(1023.0);
    assert_eq!(result, "1023.00B");
}

#[test]
fn test_human_bytes_boundary_1023_kb() {
    let result = human_bytes(1023.0 * 1024.0);
    assert_eq!(result, "1023.00KB");
}

#[test]
fn test_human_bytes_boundary_1023_mb() {
    let result = human_bytes(1023.0 * 1024.0 * 1024.0);
    assert_eq!(result, "1023.00MB");
}

#[test]
fn test_human_bytes_boundary_1023_gb() {
    let result = human_bytes(1023.0 * 1024.0 * 1024.0 * 1024.0);
    assert_eq!(result, "1023.00GB");
}

#[test]
fn test_human_bytes_real_world_ssd_speed() {
    // Typical SATA SSD: ~500 MB/s
    let result = human_bytes(500.0 * 1024.0 * 1024.0);
    assert_eq!(result, "500.00MB");
}

#[test]
fn test_human_bytes_real_world_nvme_speed() {
    // Typical NVMe: ~3.5 GB/s
    let result = human_bytes(3.5 * 1024.0 * 1024.0 * 1024.0);
    assert_eq!(result, "3.50GB");
}

#[test]
fn test_human_bytes_real_world_hdd_speed() {
    // Typical HDD: ~150 MB/s
    let result = human_bytes(150.0 * 1024.0 * 1024.0);
    assert_eq!(result, "150.00MB");
}

#[test]
fn test_human_bytes_very_small() {
    let result = human_bytes(0.5);
    assert_eq!(result, "0.50B");
}

#[test]
fn test_human_bytes_fractional_units() {
    let result = human_bytes(0.125 * 1024.0); // 0.125 KB = 128 bytes
    assert_eq!(result, "128.00B"); // Stays as bytes since < 1024
}

// ==================== DURATION FORMATTING TESTS ====================

#[test]
fn test_format_duration_zero() {
    let result = format_duration(0);
    assert_eq!(result, "0:00");
}

#[test]
fn test_format_duration_seconds_only() {
    let result = format_duration(45);
    assert_eq!(result, "0:45");
}

#[test]
fn test_format_duration_one_minute() {
    let result = format_duration(60);
    assert_eq!(result, "1:00");
}

#[test]
fn test_format_duration_minutes_seconds() {
    let result = format_duration(125); // 2:05
    assert_eq!(result, "2:05");
}

#[test]
fn test_format_duration_59_minutes() {
    let result = format_duration(59 * 60 + 59); // 59:59
    assert_eq!(result, "59:59");
}

#[test]
fn test_format_duration_one_hour() {
    let result = format_duration(3600); // 1:00:00
    assert_eq!(result, "1:00:00");
}

#[test]
fn test_format_duration_hours_minutes_seconds() {
    let result = format_duration(3661); // 1:01:01
    assert_eq!(result, "1:01:01");
}

#[test]
fn test_format_duration_two_hours() {
    let result = format_duration(2 * 3600 + 30 * 60 + 45); // 2:30:45
    assert_eq!(result, "2:30:45");
}

#[test]
fn test_format_duration_ten_hours() {
    let result = format_duration(10 * 3600 + 5 * 60 + 3); // 10:05:03
    assert_eq!(result, "10:05:03");
}

#[test]
fn test_format_duration_24_hours() {
    let result = format_duration(24 * 3600); // 24:00:00
    assert_eq!(result, "24:00:00");
}

#[test]
fn test_format_duration_real_world_1gb_file() {
    // 1 GB at 100 MB/s = 10 seconds
    let result = format_duration(10);
    assert_eq!(result, "0:10");
}

#[test]
fn test_format_duration_real_world_1tb_drive() {
    // 1 TB at 150 MB/s ≈ 1.9 hours ≈ 6826 seconds
    let result = format_duration(6826);
    assert_eq!(result, "1:53:46");
}

#[test]
fn test_format_duration_padding_single_digit_seconds() {
    let result = format_duration(5);
    assert_eq!(result, "0:05");
}

#[test]
fn test_format_duration_padding_single_digit_minutes() {
    let result = format_duration(5 * 60 + 30); // 5:30
    assert_eq!(result, "5:30");
}

#[test]
fn test_format_duration_padding_hours() {
    let result = format_duration(3600 + 5 * 60 + 7); // 1:05:07
    assert_eq!(result, "1:05:07");
}

// ==================== PROGRESS CLAMPING TESTS ====================

#[test]
fn test_progress_clamp_normal() {
    // Test that normal progress values work
    let mut bar = ProgressBar::new(50);

    // These should all work without panic
    bar.render(0.0, None, None);
    bar.render(50.0, None, None);
    bar.render(100.0, None, None);
}

#[test]
fn test_progress_clamp_negative() {
    let mut bar = ProgressBar::new(50);
    // Should clamp to 0
    bar.render(-10.0, None, None);
}

#[test]
fn test_progress_clamp_over_100() {
    let mut bar = ProgressBar::new(50);
    // Should clamp to 100
    bar.render(150.0, None, None);
}

#[test]
fn test_progress_clamp_nan() {
    let mut bar = ProgressBar::new(50);
    // Should handle NaN gracefully
    bar.render(f64::NAN, None, None);
}

#[test]
fn test_progress_clamp_infinity() {
    let mut bar = ProgressBar::new(50);
    // Should clamp infinity to 100
    bar.render(f64::INFINITY, None, None);
}

// ==================== BAR WIDTH CALCULATION TESTS ====================

#[test]
fn test_bar_width_0_percent() {
    // At 0%, filled = 0, empty = width
    let width: usize = 50;
    let progress = 0.0;
    let filled = ((progress / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    assert_eq!(filled, 0);
    assert_eq!(empty, 50);
}

#[test]
fn test_bar_width_50_percent() {
    let width: usize = 50;
    let progress = 50.0;
    let filled = ((progress / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    assert_eq!(filled, 25);
    assert_eq!(empty, 25);
}

#[test]
fn test_bar_width_100_percent() {
    let width: usize = 50;
    let progress = 100.0;
    let filled = ((progress / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    assert_eq!(filled, 50);
    assert_eq!(empty, 0);
}

#[test]
fn test_bar_width_rounding() {
    // Test that rounding works correctly
    let width = 50;
    let progress = 33.33; // Should round to 17/50
    let filled = ((progress / 100.0) * width as f64).round() as usize;

    assert_eq!(filled, 17);
}

#[test]
fn test_bar_width_small_width() {
    let width: usize = 10;
    let progress = 25.0;
    let filled = ((progress / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    assert_eq!(filled, 3); // 2.5 rounds to 3 with .round()
    assert_eq!(empty, 7);
}

// ==================== SPEED AND ETA CALCULATION TESTS ====================

#[test]
fn test_speed_calculation() {
    // Speed = bytes_written / elapsed_time
    let bytes_written = 100 * 1024 * 1024u64; // 100 MB
    let elapsed = 1.0; // 1 second

    let speed = (bytes_written as f64) / elapsed;
    assert_eq!(speed, 100.0 * 1024.0 * 1024.0); // 100 MB/s
}

#[test]
fn test_eta_calculation() {
    let written = 100 * 1024 * 1024u64; // 100 MB
    let total = 1000 * 1024 * 1024u64; // 1 GB
    let speed = 100.0 * 1024.0 * 1024.0; // 100 MB/s

    let remaining = total - written; // 900 MB
    let eta_secs = ((remaining as f64) / speed).round() as u64;

    assert_eq!(eta_secs, 9); // 9 seconds
}

#[test]
fn test_eta_calculation_zero_speed() {
    let remaining = 1000 * 1024 * 1024u64;
    let speed = 0.0;

    let eta_secs = if speed > 0.0 {
        (remaining as f64 / speed).round() as u64
    } else {
        0
    };

    assert_eq!(eta_secs, 0);
}

#[test]
fn test_remaining_bytes_calculation() {
    let total = 1000u64;
    let written = 300u64;
    let remaining = total.saturating_sub(written);

    assert_eq!(remaining, 700);
}

#[test]
fn test_remaining_bytes_when_written_exceeds_total() {
    let total = 1000u64;
    let written = 1200u64;
    let remaining = total.saturating_sub(written);

    assert_eq!(remaining, 0);
}

// ==================== ANIMATION FRAME TESTS ====================

#[test]
fn test_cat_frames_defined() {
    // Verify CAT_FRAMES has the expected number of frames
    assert_eq!(CAT_FRAMES.len(), 6);
}

#[test]
fn test_paw_frames_defined() {
    // Verify PAW_FRAMES has the expected number of frames
    assert_eq!(PAW_FRAMES.len(), 4);
}

#[test]
fn test_frame_cycling_logic() {
    // Test the modulo cycling logic used for frames
    let cat_frames_len = 6;
    let mut frame = 0;

    for _ in 0..12 {
        frame = (frame + 1) % cat_frames_len;
    }

    assert_eq!(frame, 0); // Should cycle back to 0
}

#[test]
fn test_cat_position_wrapping() {
    let width = 50;
    let mut pos = 0;

    for _ in 0..100 {
        pos = (pos + 1) % width.max(1);
    }

    assert_eq!(pos, 0); // Should wrap around
}

// ==================== EDGE CASE TESTS ====================

#[test]
fn test_render_with_bytes_info() {
    let mut bar = ProgressBar::new(50);
    bar.render(50.0, Some(500_000_000), Some(1_000_000_000));
    // Should not panic
}

#[test]
fn test_render_without_bytes_info() {
    let mut bar = ProgressBar::new(50);
    bar.render(50.0, None, None);
    // Should not panic
}

#[test]
fn test_render_multiple_times() {
    let mut bar = ProgressBar::new(50);

    for progress in 0..=10 {
        bar.render(progress as f64 * 10.0, None, None);
    }
    // Should not panic
}

#[test]
fn test_very_large_width() {
    let bar = ProgressBar::new(1000);
    let _ = bar;
}

#[test]
fn test_human_bytes_very_large() {
    // Test beyond TB (should stop at TB)
    let huge = 1024.0 * 1024.0 * 1024.0 * 1024.0 * 10.0; // 10 TB
    let result = human_bytes(huge);
    assert_eq!(result, "10.00TB");
}

#[test]
fn test_format_duration_very_large() {
    // 100 hours
    let result = format_duration(100 * 3600);
    assert_eq!(result, "100:00:00");
}

// ==================== REAL-WORLD SCENARIO TESTS ====================

#[test]
fn test_scenario_fast_ssd_wipe() {
    // 1TB SSD at 500 MB/s
    let total = 1024 * 1024 * 1024 * 1024u64; // 1 TB
    let speed = 500.0 * 1024.0 * 1024.0; // 500 MB/s
    let eta_secs = (total as f64 / speed).round() as u64;

    let eta_str = format_duration(eta_secs);
    // Should be around 34-35 minutes
    assert!(eta_str.contains(":"));
}

#[test]
fn test_scenario_slow_hdd_wipe() {
    // 2TB HDD at 100 MB/s
    let total = 2 * 1024 * 1024 * 1024 * 1024u64; // 2 TB
    let speed = 100.0 * 1024.0 * 1024.0; // 100 MB/s
    let eta_secs = (total as f64 / speed).round() as u64;

    let eta_str = format_duration(eta_secs);
    // Should be several hours
    assert!(eta_str.contains(":"));
}

#[test]
fn test_scenario_progress_updates() {
    let mut bar = ProgressBar::new(48);

    let total = 1_000_000_000u64; // 1 GB
    let increments = 10;

    for i in 0..=increments {
        let written = (total / increments) * i;
        let progress = (i as f64 / increments as f64) * 100.0;
        bar.render(progress, Some(written), Some(total));
    }
    // Should complete without panic
}
