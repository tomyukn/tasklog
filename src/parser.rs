use anyhow::{anyhow, Result};
use regex::Regex;

/// Parse an `"HHMM"` or `"HH:MM"` style string to a tuple of int values which replesents
/// hours and minutes.
pub fn parse_time_hm(s: &str) -> Result<(u32, u32)> {
    let re_time = Regex::new(r"^([0-2][0-9]|[0-9]):?([0-5][0-9])$").unwrap();
    let captures = re_time.captures(s).ok_or(anyhow!("invalid time"))?;
    let h = captures.get(1).unwrap().as_str().parse::<u32>()?;
    let m = captures.get(2).unwrap().as_str().parse::<u32>()?;

    if h < 24 && m < 60 {
        Ok((h, m))
    } else {
        Err(anyhow!("invalid time"))
    }
}

/// Parse a date string to a tuple of int values which replesents year, month, and day.
pub fn parse_date(s: &str) -> Result<(i32, u32, u32)> {
    let re_ymd = Regex::new(r"(?P<year>[0-9]{4})-?(?P<month>[0-9]{2})-?(?P<day>[0-9]{2})").unwrap();

    let captures = re_ymd.captures(s).ok_or(anyhow!("invalid date"))?;
    let y = captures.name("year").unwrap().as_str().parse::<i32>()?;
    let m = captures.name("month").unwrap().as_str().parse::<u32>()?;
    let d = captures.name("day").unwrap().as_str().parse::<u32>()?;

    if m >= 1 && m <= 12 && d >= 1 && d <= 31 {
        Ok((y, m, d))
    } else {
        Err(anyhow!("invalid date"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hhmm() {
        assert_eq!(parse_time_hm("2310").unwrap(), (23, 10));
        assert_eq!(parse_time_hm("0559").unwrap(), (5, 59));
        assert_eq!(parse_time_hm("0605").unwrap(), (6, 5));
        assert_eq!(parse_time_hm("23:10").unwrap(), (23, 10));
        assert_eq!(parse_time_hm("05:59").unwrap(), (5, 59));
        assert_eq!(parse_time_hm("6:05").unwrap(), (6, 5));

        assert!(parse_time_hm("aaa").is_err());
        assert!(parse_time_hm("2410").is_err());
        assert!(parse_time_hm("0560").is_err());
        assert!(parse_time_hm("24:10").is_err());
        assert!(parse_time_hm("05:60").is_err());
        assert!(parse_time_hm("5:60").is_err());
    }

    #[test]
    fn test_parse_date() {
        assert_eq!(parse_date("2021-01-01").unwrap(), (2021, 1, 1));
        assert_eq!(parse_date("2021-12-31").unwrap(), (2021, 12, 31));
        assert_eq!(parse_date("20210101").unwrap(), (2021, 1, 1));
        assert_eq!(parse_date("20211231").unwrap(), (2021, 12, 31));

        assert!(parse_date("2021-00-01").is_err());
        assert!(parse_date("2021-13-31").is_err());
        assert!(parse_date("20210100").is_err());
        assert!(parse_date("20211232").is_err());
    }
}
