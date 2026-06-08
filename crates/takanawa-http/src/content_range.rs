use takanawa_core::{Result, TakanawaError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentRange {
    pub start: u64,
    pub end: u64,
    pub total: u64,
}

pub fn parse_content_range(value: &str) -> Result<ContentRange> {
    let value = value.trim();
    let Some(rest) = value.strip_prefix("bytes ") else {
        return Err(TakanawaError::HttpProtocol(format!(
            "Content-Range does not use bytes unit: {value}"
        )));
    };
    if rest.starts_with("*/") {
        return Err(TakanawaError::HttpProtocol(format!(
            "unsatisfied Content-Range is not a byte range: {value}"
        )));
    }

    let (range, total) = rest
        .split_once('/')
        .ok_or_else(|| TakanawaError::HttpProtocol(format!("invalid Content-Range: {value}")))?;
    if total == "*" {
        return Err(TakanawaError::HttpProtocol(
            "Content-Range total length is unknown".to_owned(),
        ));
    }
    let total = total
        .parse::<u64>()
        .map_err(|err| TakanawaError::HttpProtocol(format!("invalid total length: {err}")))?;
    let (start, end) = range
        .split_once('-')
        .ok_or_else(|| TakanawaError::HttpProtocol(format!("invalid range: {value}")))?;
    let start = start
        .parse::<u64>()
        .map_err(|err| TakanawaError::HttpProtocol(format!("invalid range start: {err}")))?;
    let end = end
        .parse::<u64>()
        .map_err(|err| TakanawaError::HttpProtocol(format!("invalid range end: {err}")))?;
    if start > end || end >= total {
        return Err(TakanawaError::HttpProtocol(format!(
            "invalid Content-Range bounds: {value}"
        )));
    }

    Ok(ContentRange { start, end, total })
}

pub fn parse_unsatisfied_total(value: &str) -> Result<u64> {
    let value = value.trim();
    let Some(rest) = value.strip_prefix("bytes */") else {
        return Err(TakanawaError::HttpProtocol(format!(
            "invalid unsatisfied Content-Range: {value}"
        )));
    };
    rest.parse::<u64>()
        .map_err(|err| TakanawaError::HttpProtocol(format!("invalid unsatisfied total: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_content_range() {
        let parsed = parse_content_range("bytes 4-7/10").unwrap();

        assert_eq!(
            parsed,
            ContentRange {
                start: 4,
                end: 7,
                total: 10,
            }
        );
    }

    #[test]
    fn rejects_unknown_total() {
        assert!(parse_content_range("bytes 0-1/*").is_err());
    }

    #[test]
    fn parses_unsatisfied_total() {
        assert_eq!(parse_unsatisfied_total("bytes */0").unwrap(), 0);
    }
}
