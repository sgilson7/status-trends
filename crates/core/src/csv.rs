//! Reading a delimited text export: CSV from Jira or a spreadsheet, or tab-
//! separated text pasted from one. RFC 4180 quoting: a quoted field may hold
//! the delimiter, a line break, or a doubled quote.

/// A header row and the rows under it, each a list of fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Table {
    /// The field in `row` at `column`, or "" when the row is short.
    pub fn cell(&self, row: usize, column: usize) -> &str {
        self.rows
            .get(row)
            .and_then(|r| r.get(column))
            .map(|s| s.as_str())
            .unwrap_or("")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsvError {
    Empty,
    NoRows,
    UnclosedQuote { line: usize },
}

impl CsvError {
    pub fn message(&self) -> String {
        match self {
            CsvError::Empty => "The file is empty.".to_string(),
            CsvError::NoRows => {
                "The file has a header row but no rows under it.".to_string()
            }
            CsvError::UnclosedQuote { line } => format!(
                "A quoted field that starts on line {line} is never closed, so the file cannot be read as CSV."
            ),
        }
    }
}

/// The delimiter used by the first line: whichever of comma, semicolon and
/// tab appears most often outside quotes. Comma when none appears.
pub fn detect_delimiter(text: &str) -> char {
    let mut counts = [0usize; 3];
    let mut quoted = false;
    for c in text.chars() {
        match c {
            '"' => quoted = !quoted,
            '\n' | '\r' if !quoted => break,
            ',' if !quoted => counts[0] += 1,
            ';' if !quoted => counts[1] += 1,
            '\t' if !quoted => counts[2] += 1,
            _ => {}
        }
    }
    let mut best = ',';
    let mut most = counts[0];
    if counts[1] > most {
        best = ';';
        most = counts[1];
    }
    if counts[2] > most {
        best = '\t';
    }
    best
}

/// Splits text into records. Blank lines are skipped. Fields are kept as
/// written, except that the quotes around a quoted field are removed.
pub fn records(text: &str, delimiter: char) -> Result<Vec<Vec<String>>, CsvError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut record: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut quote_line = 0;
    let mut line = 1;
    let mut chars = text.chars().peekable();
    // A record that is one empty field is a blank line.
    let push_record = |record: &mut Vec<String>, out: &mut Vec<Vec<String>>| {
        let blank = record.len() == 1 && record[0].trim().is_empty();
        if !blank {
            out.push(std::mem::take(record));
        } else {
            record.clear();
        }
    };
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                if c == '\n' {
                    line += 1;
                }
                field.push(c);
            }
            continue;
        }
        if c == '"' && field.is_empty() {
            quoted = true;
            quote_line = line;
        } else if c == delimiter {
            record.push(std::mem::take(&mut field));
        } else if c == '\r' || c == '\n' {
            if c == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
            }
            line += 1;
            record.push(std::mem::take(&mut field));
            push_record(&mut record, &mut out);
        } else {
            field.push(c);
        }
    }
    if quoted {
        return Err(CsvError::UnclosedQuote { line: quote_line });
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        push_record(&mut record, &mut out);
    }
    Ok(out)
}

/// Reads the text as a table whose first record is the header. Header names
/// are trimmed; an empty one becomes "Column N".
pub fn parse(text: &str) -> Result<Table, CsvError> {
    if text.trim().is_empty() {
        return Err(CsvError::Empty);
    }
    let mut all = records(text, detect_delimiter(text))?;
    if all.is_empty() {
        return Err(CsvError::Empty);
    }
    let header: Vec<String> = all
        .remove(0)
        .into_iter()
        .enumerate()
        .map(|(i, h)| {
            let h = h.trim().to_string();
            if h.is_empty() {
                format!("Column {}", i + 1)
            } else {
                h
            }
        })
        .collect();
    if all.is_empty() {
        return Err(CsvError::NoRows);
    }
    Ok(Table { header, rows: all })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_rows_are_split_on_commas() {
        let t = parse("a,b\n1,2\n3,4\n").unwrap();
        assert_eq!(t.header, vec!["a", "b"]);
        assert_eq!(t.rows, vec![vec!["1", "2"], vec!["3", "4"]]);
    }

    #[test]
    fn quoted_fields_keep_commas_quotes_and_line_breaks() {
        let t = parse("k,s\r\nT-1,\"Fix \"\"login\"\", again\r\nand more\"\r\nT-2,x").unwrap();
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.rows[0][1], "Fix \"login\", again\r\nand more");
        assert_eq!(t.rows[1], vec!["T-2", "x"]);
    }

    #[test]
    fn a_byte_order_mark_and_blank_lines_are_ignored() {
        let t = parse("\u{feff}a,b\n\n1,2\n\n").unwrap();
        assert_eq!(t.header, vec!["a", "b"]);
        assert_eq!(t.rows, vec![vec!["1", "2"]]);
    }

    #[test]
    fn semicolons_and_tabs_are_detected() {
        assert_eq!(detect_delimiter("a;b;c\n1,5;2;3"), ';');
        assert_eq!(detect_delimiter("a\tb\n1\t2"), '\t');
        assert_eq!(detect_delimiter("\"a;b\",c\n"), ',');
        assert_eq!(detect_delimiter("single"), ',');
        let t = parse("a\tb\n1\t2").unwrap();
        assert_eq!(t.rows[0], vec!["1", "2"]);
    }

    #[test]
    fn short_rows_read_as_empty_cells() {
        let t = parse("a,b,c\n1\n").unwrap();
        assert_eq!(t.cell(0, 0), "1");
        assert_eq!(t.cell(0, 2), "");
        assert_eq!(t.cell(5, 0), "");
    }

    #[test]
    fn empty_header_names_are_numbered() {
        let t = parse("a,,c\n1,2,3").unwrap();
        assert_eq!(t.header, vec!["a", "Column 2", "c"]);
    }

    #[test]
    fn broken_files_are_refused_with_a_message() {
        assert_eq!(parse("  \n "), Err(CsvError::Empty));
        assert_eq!(parse("a,b\n"), Err(CsvError::NoRows));
        let e = parse("a,b\n1,\"open\n2,3").unwrap_err();
        assert_eq!(e, CsvError::UnclosedQuote { line: 2 });
        assert!(e.message().contains("line 2"));
    }
}
