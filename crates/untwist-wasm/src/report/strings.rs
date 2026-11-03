use serde::Serialize;
use untwist::DecodedSite;

use super::schema::Utf16Index;

pub const DECODED_ROWS_CAP: usize = 200_000;

type Row<'a> = (Option<u32>, Option<u32>, &'a str);

#[derive(Serialize)]
struct Table<'a> {
    rows: Vec<Row<'a>>,
    more: usize,
}

fn row<'a>(site: &'a DecodedSite, index: &Utf16Index) -> Row<'a> {
    let span = site
        .span
        .filter(|_| site.layer == 0)
        .and_then(|(start, end)| Some((index.at(start)?, index.at(end)?)));
    (
        span.map(|span| span.0),
        span.map(|span| span.1),
        &site.value,
    )
}

pub fn decoded_json(sites: &[DecodedSite], source: &str) -> String {
    if sites.is_empty() {
        return String::new();
    }
    let index = Utf16Index::new(source, true);
    let table = Table {
        rows: sites
            .iter()
            .take(DECODED_ROWS_CAP)
            .map(|site| row(site, &index))
            .collect(),
        more: sites.len().saturating_sub(DECODED_ROWS_CAP),
    };
    serde_json::to_string(&table).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::decoded_json;
    use untwist::DecodedSite;

    fn site(layer: u32, span: Option<(u32, u32)>, value: &str) -> DecodedSite {
        DecodedSite {
            input: 0,
            layer,
            span,
            value: value.to_string(),
        }
    }

    #[test]
    fn rows_carry_utf16_spans_into_the_input_and_none_for_other_layers() {
        let source = "\u{e9}b(0);c(1);";
        let json = decoded_json(
            &[
                site(0, Some((2, 6)), "He"),
                site(1, Some((0, 1)), "llo"),
                site(0, None, "x"),
            ],
            source,
        );
        assert_eq!(
            json,
            r#"{"rows":[[1,5,"He"],[null,null,"llo"],[null,null,"x"]],"more":0}"#
        );
        assert_eq!(decoded_json(&[], source), "");
    }
}
