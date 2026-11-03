use untwist_deobfuscate::decoded_strings::DecodedString;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedSite {
    pub input: usize,
    pub layer: u32,
    pub span: Option<(u32, u32)>,
    pub value: String,
}

pub(super) fn sited(text_len: usize, layer: u32, found: Vec<DecodedString>) -> Vec<DecodedSite> {
    found
        .into_iter()
        .map(|found| {
            let (start, end) = (found.span.start, found.span.end);
            let inside = start <= end && usize::try_from(end).is_ok_and(|end| end <= text_len);
            DecodedSite {
                input: 0,
                layer,
                span: inside.then_some((start, end)),
                value: found.value,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_span::Span;

    fn found(start: u32, end: u32, value: &str) -> DecodedString {
        DecodedString {
            span: Span::new(start, end),
            value: value.to_string(),
        }
    }

    #[test]
    fn a_site_keeps_its_span_only_when_it_lies_in_the_text() {
        let sites = sited(10, 1, vec![found(2, 6, "a"), found(8, 12, "b")]);
        assert_eq!(
            sites,
            [
                DecodedSite {
                    input: 0,
                    layer: 1,
                    span: Some((2, 6)),
                    value: "a".to_string()
                },
                DecodedSite {
                    input: 0,
                    layer: 1,
                    span: None,
                    value: "b".to_string()
                },
            ]
        );
    }
}
