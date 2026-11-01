use oxc_diagnostics::OxcDiagnostic;

pub fn format_diagnostic(source: &str, diagnostic: &OxcDiagnostic) -> String {
    let message = diagnostic.to_string();
    let label = diagnostic
        .labels
        .iter()
        .find(|label| label.primary())
        .or_else(|| diagnostic.labels.first());
    let Some(label) = label else {
        return message;
    };
    match line_column(source, label.offset() as usize) {
        Some((line, column)) => format!("{message} ({line}:{column})"),
        None => message,
    }
}

fn line_column(source: &str, offset: usize) -> Option<(usize, usize)> {
    let offset = offset.min(source.len());
    let prefix = source.get(..offset)?;
    let line = prefix.matches('\n').count() + 1;
    let column = match prefix.rfind('\n') {
        Some(pos) => prefix.get(pos + 1..)?.chars().count() + 1,
        None => prefix.chars().count() + 1,
    };
    Some((line, column))
}
