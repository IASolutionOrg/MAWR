use encoding_rs::{Encoding, UTF_8};

pub(crate) struct DecodedHtml {
    pub(crate) text: String,
    pub(crate) had_replacements: bool,
}

pub(crate) fn decode(bytes: &[u8], content_type: Option<&str>) -> DecodedHtml {
    let encoding = bom_encoding(bytes)
        .or_else(|| content_type.and_then(charset_from_content_type))
        .or_else(|| charset_from_meta(bytes))
        .unwrap_or(UTF_8);
    let (text, _, had_replacements) = encoding.decode(bytes);
    DecodedHtml {
        text: text.into_owned(),
        had_replacements,
    }
}

fn bom_encoding(bytes: &[u8]) -> Option<&'static Encoding> {
    Encoding::for_bom(bytes).map(|(encoding, _)| encoding)
}

fn charset_from_content_type(content_type: &str) -> Option<&'static Encoding> {
    content_type
        .split(';')
        .skip(1)
        .find_map(|parameter| {
            let (name, value) = parameter.split_once('=')?;
            name.trim()
                .eq_ignore_ascii_case("charset")
                .then(|| value.trim().trim_matches(['\'', '"']).as_bytes())
        })
        .and_then(Encoding::for_label)
}

fn charset_from_meta(bytes: &[u8]) -> Option<&'static Encoding> {
    let prefix = &bytes[..bytes.len().min(1_024)];
    let ascii = String::from_utf8_lossy(prefix).to_ascii_lowercase();
    let charset = ascii.find("charset")?;
    let after = ascii.get(charset + "charset".len()..)?.trim_start();
    let after = after.strip_prefix('=')?.trim_start();
    let label = after
        .trim_start_matches(['\'', '"'])
        .split(|character: char| {
            character.is_ascii_whitespace() || matches!(character, '\'' | '"' | '>' | ';')
        })
        .next()?;
    Encoding::for_label(label.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn honors_http_charset_and_reports_replacements() {
        let decoded = decode(
            &[0x63, 0x61, 0x66, 0xe9],
            Some("text/html; charset=windows-1252"),
        );
        assert_eq!(decoded.text, "café");
        assert!(!decoded.had_replacements);

        assert!(decode(&[0xff], Some("text/html; charset=utf-8")).had_replacements);
    }

    #[test]
    fn honors_early_meta_charset_without_transport_metadata() {
        let mut html = b"<meta charset=windows-1252><p>caf".to_vec();
        html.push(0xe9);
        html.extend_from_slice(b"</p>");
        assert!(decode(&html, None).text.contains("café"));
    }
}
