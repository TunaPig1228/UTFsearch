// Quick diagnostic for mtime parsing issue
// Run with: rustc test_mtime_debug.rs && ./test_mtime_debug.exe

fn parse_mtime(mtime_str: &str) -> i64 {
    if mtime_str.is_empty() || mtime_str == "0" {
        return 0;
    }
    
    let parts: Vec<&str> = mtime_str.split_whitespace().collect();
    if parts.len() < 2 {
        eprintln!("  Split parts: {:?} (len={})", parts, parts.len());
        return 0;
    }
    
    let date_parts: Vec<&str> = parts[0].split('/').collect();
    if date_parts.len() < 3 {
        eprintln!("  Date parts: {:?} (len={})", date_parts, date_parts.len());
        return 0;
    }
    
    let year = date_parts[0].parse::<i32>().unwrap_or(1970);
    let month = date_parts[1].parse::<u32>().unwrap_or(1);
    let day = date_parts[2].parse::<u32>().unwrap_or(1);
    
    let time_parts: Vec<&str> = parts[1].split(':').collect();
    let hour = if time_parts.len() > 0 { time_parts[0].parse::<u32>().unwrap_or(0) } else { 0 };
    let minute = if time_parts.len() > 1 { time_parts[1].parse::<u32>().unwrap_or(0) } else { 0 };
    let second = if time_parts.len() > 2 { time_parts[2].parse::<u32>().unwrap_or(0) } else { 0 };
    
    eprintln!("  Parsed: year={}, month={}, day={}, hour={}, minute={}, second={}", 
        year, month, day, hour, minute, second);
    
    let days_since_2000 = days_since_epoch(year, month, day);
    eprintln!("  days_since_2000={}", days_since_2000);
    
    let seconds = (days_since_2000 as i64) * 86400 + (hour as i64 * 3600) + (minute as i64 * 60) + (second as i64);
    eprintln!("  seconds_since_2000={}", seconds);
    eprintln!("  seconds_adjustment=946684800");
    
    let result = 946684800 + seconds; // FIXED: should be + not -
    eprintln!("  result={}", result);
    result
}

fn days_since_epoch(year: i32, month: u32, day: u32) -> i32 {
    let mut days = 0;
    
    eprintln!("    days_since_epoch: year={}", year);
    
    // Add days for complete years
    for y in 2000..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }
    eprintln!("    after year loop: days={}", days);
    
    // Add days for complete months in current year
    let days_in_month = [31, if is_leap_year(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 1..month {
        days += days_in_month[(m - 1) as usize] as i32;
    }
    eprintln!("    after month loop: days={}", days);
    
    // Add remaining days
    days += day as i32;
    eprintln!("    after day add: days={}", days);
    
    days
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn main() {
    println!("Testing parse_mtime function:\n");
    
    let test_cases = vec![
        "2024/01/01 00:00:00",   // Recent past
        "2026/08/20 12:00:00",   // Today-ish
        "2025/06/15 10:30:45",   // Recent
        "2000/01/01 00:00:00",   // Unix epoch +30 years
        "1970/01/01 00:00:00",   // Unix epoch (should fail?)
        "2024/12/31 23:59:59",   // Year end
        "19/08/20 12:00:00",     // Year truncated (bad format!)
        "",                       // Empty
    ];
    
    for test in test_cases {
        println!("Input: '{}'", test);
        let result = parse_mtime(test);
        println!("Result: {} ({})\n", result, 
            if result < 0 { "NEGATIVE!" } else { "ok" });
    }
    
    // Now test what the real catalog mtime means
    println!("\n=== Reverse: What timestamp is -106153655? ===");
    let unix_ts = -106153655i64 + 946684800i64;
    println!("Unix timestamp: {} (year ~{})", unix_ts, 1970 + unix_ts / (365 * 86400));
    // This is roughly 1996
    
    println!("\n=== Testing if year is truncated ===");
    println!("If '24/08/20 12:00:00' (year=24, which is < 2000):");
    let result = parse_mtime("24/08/20 12:00:00");
    println!("Result: {}\n", result);
}
