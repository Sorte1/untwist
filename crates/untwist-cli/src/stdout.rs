use std::io::{self, Write};

pub fn write_codes(out: &mut dyn Write, codes: &[String]) -> io::Result<()> {
    for (index, code) in codes.iter().enumerate() {
        if index > 0 {
            out.write_all(b"\n\n")?;
        }
        out.write_all(code.as_bytes())?;
    }
    out.write_all(b"\n")
}

pub fn write_line(out: &mut dyn Write, text: &str) -> io::Result<()> {
    out.write_all(text.as_bytes())?;
    out.write_all(b"\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_joined_by_a_blank_line_and_end_with_one_newline() {
        let mut out = Vec::new();
        write_codes(
            &mut out,
            &["a;".to_string(), String::new(), "b;".to_string()],
        )
        .unwrap();
        assert_eq!(out, b"a;\n\n\n\nb;\n");
    }

    #[test]
    fn a_line_ends_with_one_newline() {
        let mut out = Vec::new();
        write_line(&mut out, "{}").unwrap();
        assert_eq!(out, b"{}\n");
    }
}
