//! Human-readable formatting of sizes, percentages, dates and modes.

/// Binary-prefixed size, e.g. `12.3 MiB`.
pub fn size(bytes: u64) -> String {
    const UNITS: [&str; 7] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut v = bytes as f64;
    let mut unit = 0;
    while v >= 1024.0 && unit < UNITS.len() - 1 {
        v /= 1024.0;
        unit += 1;
    }
    if v >= 100.0 {
        format!("{v:.0} {}", UNITS[unit])
    } else {
        format!("{v:.1} {}", UNITS[unit])
    }
}

/// Signed size delta, e.g. `+1.5 MiB`.
pub fn delta(d: i64) -> String {
    let sign = if d < 0 { '-' } else { '+' };
    format!("{sign}{}", size(d.unsigned_abs()))
}

/// Share of `part` in `whole` as a fraction in `0..=1`.
pub fn fraction(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        0.0
    } else {
        (part as f64 / whole as f64).clamp(0.0, 1.0)
    }
}

/// Percentage with one decimal, e.g. ` 42.0%`.
pub fn percent(f: f64) -> String {
    format!("{:5.1}%", f * 100.0)
}

/// `ls -l` style mode string, e.g. `drwxr-xr-x`.
pub fn mode(mode: u32) -> String {
    let kind = match mode & 0o170000 {
        0o040000 => 'd',
        0o120000 => 'l',
        0o020000 => 'c',
        0o060000 => 'b',
        0o010000 => 'p',
        0o140000 => 's',
        _ => '-',
    };
    let mut s = String::with_capacity(10);
    s.push(kind);
    let bits = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    for (bit, ch) in bits {
        s.push(if mode & bit != 0 { ch } else { '-' });
    }
    format!("{s} ({:04o})", mode & 0o7777)
}

/// UTC date `YYYY-MM-DD HH:MM` from seconds since the epoch.
pub fn date(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02} UTC",
        rem / 3600,
        rem % 3600 / 60
    )
}

/// Howard Hinnant's days-to-civil algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Parses `10M`, `1.5G`, `200k` or a plain byte count (binary multiples).
pub fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    let trimmed = lower
        .trim_end_matches("ib")
        .trim_end_matches('b')
        .trim_end();
    let (num, mult) = match trimmed.chars().last()? {
        'k' => (&trimmed[..trimmed.len() - 1], 1u64 << 10),
        'm' => (&trimmed[..trimmed.len() - 1], 1 << 20),
        'g' => (&trimmed[..trimmed.len() - 1], 1 << 30),
        't' => (&trimmed[..trimmed.len() - 1], 1 << 40),
        _ => (trimmed, 1),
    };
    let v: f64 = num.trim().parse().ok()?;
    (v >= 0.0).then_some((v * mult as f64) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(size(0), "0 B");
        assert_eq!(size(1023), "1023 B");
        assert_eq!(size(1024), "1.0 KiB");
        assert_eq!(size(1536 * 1024), "1.5 MiB");
        assert_eq!(size(500 * 1024 * 1024 * 1024), "500 GiB");
        assert_eq!(delta(-2048), "-2.0 KiB");
    }

    #[test]
    fn modes_and_dates() {
        assert_eq!(mode(0o040755), "drwxr-xr-x (0755)");
        assert_eq!(mode(0o100600), "-rw------- (0600)");
        assert_eq!(date(0), "1970-01-01 00:00 UTC");
        assert_eq!(date(1_700_000_000), "2023-11-14 22:13 UTC");
    }

    #[test]
    fn parses_sizes() {
        assert_eq!(parse_size("10M"), Some(10 << 20));
        assert_eq!(parse_size("1.5g"), Some(3 << 29));
        assert_eq!(parse_size("200 KiB"), Some(200 << 10));
        assert_eq!(parse_size("42"), Some(42));
        assert_eq!(parse_size("abc"), None);
        assert_eq!(parse_size(""), None);
    }
}
