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

/// Local date `YYYY-MM-DD HH:MM` from seconds since the epoch.
pub fn date(secs: i64) -> String {
    date_in(secs, jiff::tz::TimeZone::system())
}

/// Date in a given time zone.
pub fn date_in(secs: i64, tz: jiff::tz::TimeZone) -> String {
    match jiff::Timestamp::from_second(secs) {
        Ok(ts) => ts
            .to_zoned_datetime(tz)
            .strftime("%Y-%m-%d %H:%M")
            .to_string(),
        Err(_) => "?".into(),
    }
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
        let utc = jiff::tz::TimeZone::UTC;
        assert_eq!(date_in(0, utc.clone()), "1970-01-01 00:00");
        assert_eq!(date_in(1_700_000_000, utc), "2023-11-14 22:13");
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
