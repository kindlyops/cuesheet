//! Minimal read-only SQLite file reader: enough of the file format
//! (https://www.sqlite.org/fileformat2.html) to full-scan the small tables in
//! a playlist database. Pure Rust so the core compiles to wasm32 — no C
//! SQLite, no temp files.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl Value {
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Real(f) => Some(*f as i64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Table {
    columns: HashMap<String, usize>,
    pub rows: Vec<Vec<Value>>,
}

impl Table {
    pub fn column(&self, name: &str) -> Option<usize> {
        self.columns.get(&name.to_ascii_lowercase()).copied()
    }

    pub fn get<'a>(&self, row: &'a [Value], column: &str) -> Option<&'a Value> {
        self.column(column).and_then(|i| row.get(i))
    }
}

pub struct SqliteFile<'a> {
    data: &'a [u8],
    page_size: usize,
    usable_size: usize,
}

type SqlResult<T> = Result<T, String>;

impl<'a> SqliteFile<'a> {
    pub fn parse(data: &'a [u8]) -> SqlResult<Self> {
        if data.len() < 100 || &data[..16] != b"SQLite format 3\0" {
            return Err("not an SQLite database".to_string());
        }
        let raw = u16::from_be_bytes([data[16], data[17]]);
        let page_size = if raw == 1 { 65536 } else { raw as usize };
        if page_size < 512 || !page_size.is_power_of_two() {
            return Err(format!("invalid page size {page_size}"));
        }
        let reserved = data[20] as usize;
        Ok(SqliteFile {
            data,
            page_size,
            usable_size: page_size - reserved,
        })
    }

    /// Reads an entire table by name (full scan, in storage order).
    ///
    /// Handles both ordinary rowid tables (table b-trees) and WITHOUT ROWID
    /// tables, which SQLite stores as index b-trees keyed by the primary key
    /// columns followed by the remaining columns in declared order.
    pub fn read_table(&self, name: &str) -> SqlResult<Table> {
        let (root, sql) = self
            .find_table(name)?
            .ok_or_else(|| format!("no such table: {name}"))?;
        let columns = parse_create_table_columns(&sql);
        let ipk_index = integer_primary_key_index(&sql);
        let stored_to_declared = without_rowid_column_order(&sql, &columns);

        let mut rows = Vec::new();
        self.walk(root, &mut |rowid, mut values: Vec<Value>| {
            if let Some(order) = &stored_to_declared {
                // WITHOUT ROWID: reorder PK-first storage back to declared order.
                let mut reordered = vec![Value::Null; columns.len()];
                for (stored_idx, &declared_idx) in order.iter().enumerate() {
                    if let Some(v) = values.get(stored_idx) {
                        reordered[declared_idx] = v.clone();
                    }
                }
                values = reordered;
            } else if let Some(ipk) = ipk_index {
                if ipk < values.len() && values[ipk] == Value::Null {
                    values[ipk] = Value::Int(rowid);
                }
            }
            // Tolerate ALTER TABLE-style short records: pad with NULLs.
            while values.len() < columns.len() {
                values.push(Value::Null);
            }
            rows.push(values);
        })?;

        let columns = columns
            .into_iter()
            .enumerate()
            .map(|(i, c)| (c.to_ascii_lowercase(), i))
            .collect();
        Ok(Table { columns, rows })
    }

    fn find_table(&self, name: &str) -> SqlResult<Option<(u32, String)>> {
        // sqlite_master columns: type, name, tbl_name, rootpage, sql
        let mut found = None;
        self.walk(1, &mut |_rowid, values: Vec<Value>| {
            if found.is_some() {
                return;
            }
            let is_table = values.first().and_then(Value::as_str) == Some("table");
            let matches = values
                .get(1)
                .and_then(Value::as_str)
                .is_some_and(|n| n.eq_ignore_ascii_case(name));
            if is_table && matches {
                let root = values.get(3).and_then(Value::as_i64).unwrap_or(0) as u32;
                let sql = values
                    .get(4)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                found = Some((root, sql));
            }
        })?;
        Ok(found)
    }

    fn page(&self, number: u32) -> SqlResult<&'a [u8]> {
        let start = (number as usize - 1)
            .checked_mul(self.page_size)
            .ok_or("page offset overflow")?;
        let end = start + self.page_size;
        self.data
            .get(start..end)
            .ok_or_else(|| format!("page {number} out of range"))
    }

    /// Depth-first walk of a table b-tree, invoking `emit(rowid, values)` for
    /// every record.
    fn walk(&self, page_number: u32, emit: &mut dyn FnMut(i64, Vec<Value>)) -> SqlResult<()> {
        let page = self.page(page_number)?;
        let header_offset = if page_number == 1 { 100 } else { 0 };
        let page_type = page[header_offset];
        let cell_count =
            u16::from_be_bytes([page[header_offset + 3], page[header_offset + 4]]) as usize;

        match page_type {
            0x05 => {
                // Interior table page: 12-byte header, rightmost pointer at +8.
                let pointers = header_offset + 12;
                for i in 0..cell_count {
                    let p = pointers + i * 2;
                    let cell = u16::from_be_bytes([page[p], page[p + 1]]) as usize;
                    let child = u32::from_be_bytes([
                        page[cell],
                        page[cell + 1],
                        page[cell + 2],
                        page[cell + 3],
                    ]);
                    self.walk(child, emit)?;
                }
                let right = u32::from_be_bytes([
                    page[header_offset + 8],
                    page[header_offset + 9],
                    page[header_offset + 10],
                    page[header_offset + 11],
                ]);
                self.walk(right, emit)
            }
            0x0D => {
                // Leaf table page: 8-byte header.
                let pointers = header_offset + 8;
                for i in 0..cell_count {
                    let p = pointers + i * 2;
                    let cell = u16::from_be_bytes([page[p], page[p + 1]]) as usize;
                    let (rowid, payload) = self.leaf_cell_payload(page, cell, true)?;
                    let values = decode_record(&payload)?;
                    emit(rowid, values);
                }
                Ok(())
            }
            0x02 => {
                // Interior index page (WITHOUT ROWID tables): cells carry
                // both a left-child pointer and a key record that is itself a
                // row, plus the rightmost pointer at +8.
                let pointers = header_offset + 12;
                for i in 0..cell_count {
                    let p = pointers + i * 2;
                    let cell = u16::from_be_bytes([page[p], page[p + 1]]) as usize;
                    let child = u32::from_be_bytes([
                        page[cell],
                        page[cell + 1],
                        page[cell + 2],
                        page[cell + 3],
                    ]);
                    self.walk(child, emit)?;
                    let (_, payload) = self.leaf_cell_payload(page, cell + 4, false)?;
                    let values = decode_record(&payload)?;
                    emit(0, values);
                }
                let right = u32::from_be_bytes([
                    page[header_offset + 8],
                    page[header_offset + 9],
                    page[header_offset + 10],
                    page[header_offset + 11],
                ]);
                self.walk(right, emit)
            }
            0x0A => {
                // Leaf index page (WITHOUT ROWID tables): payload only.
                let pointers = header_offset + 8;
                for i in 0..cell_count {
                    let p = pointers + i * 2;
                    let cell = u16::from_be_bytes([page[p], page[p + 1]]) as usize;
                    let (_, payload) = self.leaf_cell_payload(page, cell, false)?;
                    let values = decode_record(&payload)?;
                    emit(0, values);
                }
                Ok(())
            }
            other => Err(format!("unsupported b-tree page type 0x{other:02x}")),
        }
    }

    /// Reads a cell payload, following overflow pages when it spills.
    /// `is_table` selects table-cell layout (payload len + rowid varints,
    /// spill threshold U-35) vs index-cell layout (payload len only,
    /// spill threshold ((U-12)*64/255)-23).
    fn leaf_cell_payload(
        &self,
        page: &[u8],
        cell: usize,
        is_table: bool,
    ) -> SqlResult<(i64, Vec<u8>)> {
        let mut pos = cell;
        let payload_len = read_varint(page, &mut pos)? as usize;
        let rowid = if is_table {
            read_varint(page, &mut pos)?
        } else {
            0
        };

        let u = self.usable_size;
        let x = if is_table {
            u - 35
        } else {
            (u - 12) * 64 / 255 - 23
        };
        let local = if payload_len <= x {
            payload_len
        } else {
            let m = (u - 12) * 32 / 255 - 23;
            let k = m + (payload_len - m) % (u - 4);
            if k <= x {
                k
            } else {
                m
            }
        };

        let mut payload = page
            .get(pos..pos + local)
            .ok_or("cell payload out of range")?
            .to_vec();

        if local < payload_len {
            let mut overflow = u32::from_be_bytes([
                page[pos + local],
                page[pos + local + 1],
                page[pos + local + 2],
                page[pos + local + 3],
            ]);
            while overflow != 0 && payload.len() < payload_len {
                let opage = self.page(overflow)?;
                overflow = u32::from_be_bytes([opage[0], opage[1], opage[2], opage[3]]);
                let take = (payload_len - payload.len()).min(self.usable_size - 4);
                payload.extend_from_slice(&opage[4..4 + take]);
            }
            if payload.len() < payload_len {
                return Err("truncated overflow chain".to_string());
            }
        }

        Ok((rowid, payload))
    }
}

/// Decodes one record (header of serial types + body).
fn decode_record(payload: &[u8]) -> SqlResult<Vec<Value>> {
    let mut pos = 0usize;
    let header_len = read_varint(payload, &mut pos)? as usize;
    let mut serials = Vec::new();
    while pos < header_len {
        serials.push(read_varint(payload, &mut pos)?);
    }

    let mut body = header_len;
    let mut values = Vec::with_capacity(serials.len());
    for serial in serials {
        let (value, size) = decode_value(payload, body, serial)?;
        values.push(value);
        body += size;
    }
    Ok(values)
}

fn decode_value(data: &[u8], at: usize, serial: i64) -> SqlResult<(Value, usize)> {
    let int_be = |n: usize| -> SqlResult<i64> {
        let bytes = data.get(at..at + n).ok_or("record body out of range")?;
        let mut v: i64 = if bytes[0] & 0x80 != 0 { -1 } else { 0 };
        for &b in bytes {
            v = (v << 8) | b as i64;
        }
        Ok(v)
    };
    Ok(match serial {
        0 => (Value::Null, 0),
        1 => (Value::Int(int_be(1)?), 1),
        2 => (Value::Int(int_be(2)?), 2),
        3 => (Value::Int(int_be(3)?), 3),
        4 => (Value::Int(int_be(4)?), 4),
        5 => (Value::Int(int_be(6)?), 6),
        6 => (Value::Int(int_be(8)?), 8),
        7 => {
            let bits = int_be(8)? as u64;
            (Value::Real(f64::from_bits(bits)), 8)
        }
        8 => (Value::Int(0), 0),
        9 => (Value::Int(1), 0),
        n if n >= 12 && n % 2 == 0 => {
            let len = ((n - 12) / 2) as usize;
            let bytes = data.get(at..at + len).ok_or("blob out of range")?;
            (Value::Blob(bytes.to_vec()), len)
        }
        n if n >= 13 => {
            let len = ((n - 13) / 2) as usize;
            let bytes = data.get(at..at + len).ok_or("text out of range")?;
            (
                Value::Text(String::from_utf8_lossy(bytes).into_owned()),
                len,
            )
        }
        other => return Err(format!("unsupported serial type {other}")),
    })
}

fn read_varint(data: &[u8], pos: &mut usize) -> SqlResult<i64> {
    let mut result: i64 = 0;
    for i in 0..9 {
        let byte = *data.get(*pos).ok_or("varint out of range")?;
        *pos += 1;
        if i == 8 {
            result = (result << 8) | byte as i64;
            return Ok(result);
        }
        result = (result << 7) | (byte & 0x7f) as i64;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
    }
    Ok(result)
}

/// Extracts column names from a CREATE TABLE statement. Handles the simple
/// schemas found in playlist databases: no quoted identifiers containing
/// commas/parens are expected, but quoted names and table constraints are.
fn parse_create_table_columns(sql: &str) -> Vec<String> {
    const CONSTRAINT_KEYWORDS: [&str; 6] = [
        "primary",
        "unique",
        "check",
        "foreign",
        "constraint",
        "without",
    ];

    let Some(open) = sql.find('(') else {
        return Vec::new();
    };
    let Some(close) = sql.rfind(')') else {
        return Vec::new();
    };
    let inner = &sql[open + 1..close];

    let mut columns = Vec::new();
    let mut depth = 0usize;
    let mut part = String::new();
    let mut parts = Vec::new();
    for ch in inner.chars() {
        match ch {
            '(' => {
                depth += 1;
                part.push(ch);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                part.push(ch);
            }
            ',' if depth == 0 => {
                parts.push(std::mem::take(&mut part));
            }
            _ => part.push(ch),
        }
    }
    parts.push(part);

    for part in parts {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let first = trimmed.split_whitespace().next().unwrap_or_default();
        let unquoted = first.trim_matches(['"', '`', '[', ']', '\'']);
        if CONSTRAINT_KEYWORDS.contains(&unquoted.to_ascii_lowercase().as_str()) {
            continue;
        }
        columns.push(unquoted.to_string());
    }
    columns
}

/// Index of the column declared `INTEGER PRIMARY KEY` (rowid alias), if any.
fn integer_primary_key_index(sql: &str) -> Option<usize> {
    let lower = sql.to_ascii_lowercase();
    let open = lower.find('(')?;
    let close = lower.rfind(')')?;
    let inner = &lower[open + 1..close];
    let mut index = 0usize;
    for part in split_top_level(inner) {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let first = trimmed.split_whitespace().next().unwrap_or_default();
        let unquoted = first.trim_matches(['"', '`', '[', ']', '\'']);
        if [
            "primary",
            "unique",
            "check",
            "foreign",
            "constraint",
            "without",
        ]
        .contains(&unquoted)
        {
            continue;
        }
        if trimmed.contains("integer") && trimmed.contains("primary key") {
            return Some(index);
        }
        index += 1;
    }
    None
}

/// For a WITHOUT ROWID table, returns the mapping from stored column position
/// to declared column position. Storage order is: primary-key columns (in PK
/// declaration order), then the remaining columns in declared order. Returns
/// None for ordinary rowid tables.
fn without_rowid_column_order(sql: &str, declared: &[String]) -> Option<Vec<usize>> {
    let lower = sql.to_ascii_lowercase();
    let close = lower.rfind(')')?;
    if !lower[close..].contains("without rowid") {
        return None;
    }

    let open = lower.find('(')?;
    let inner = &lower[open + 1..close];
    let mut pk_cols: Vec<String> = Vec::new();
    for part in split_top_level(inner) {
        let trimmed = part.trim();
        let first = trimmed
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_matches(['"', '`', '[', ']', '\'']);
        if first == "primary" || (first == "constraint" && trimmed.contains("primary key")) {
            // Table constraint: PRIMARY KEY (a, b DESC, ...)
            if let (Some(o), Some(c)) = (trimmed.find('(').map(|i| i + 1), trimmed.rfind(')')) {
                for col in trimmed[o..c].split(',') {
                    let name = col
                        .split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .trim_matches(['"', '`', '[', ']', '\'']);
                    pk_cols.push(name.to_string());
                }
            }
        } else if trimmed.contains("primary key")
            && !["unique", "check", "foreign", "without"].contains(&first)
        {
            // Inline column-level PRIMARY KEY.
            pk_cols.push(first.to_string());
        }
    }
    if pk_cols.is_empty() {
        return None;
    }

    let declared_lower: Vec<String> = declared.iter().map(|c| c.to_ascii_lowercase()).collect();
    let mut stored_to_declared: Vec<usize> = Vec::with_capacity(declared.len());
    for pk in &pk_cols {
        stored_to_declared.push(declared_lower.iter().position(|c| c == pk)?);
    }
    for (i, _) in declared_lower.iter().enumerate() {
        if !stored_to_declared.contains(&i) {
            stored_to_declared.push(i);
        }
    }
    Some(stored_to_declared)
}

fn split_top_level(s: &str) -> Vec<String> {
    let mut depth = 0usize;
    let mut part = String::new();
    let mut parts = Vec::new();
    for ch in s.chars() {
        match ch {
            '(' => {
                depth += 1;
                part.push(ch);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                part.push(ch);
            }
            ',' if depth == 0 => parts.push(std::mem::take(&mut part)),
            _ => part.push(ch),
        }
    }
    parts.push(part);
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_db(sql: &str) -> Vec<u8> {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(tmp.path()).unwrap();
        conn.execute_batch(sql).unwrap();
        // Ensure everything is in the main db file, not a -wal sidecar.
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").ok();
        drop(conn);
        std::fs::read(tmp.path()).unwrap()
    }

    #[test]
    fn reads_simple_table() {
        let db = make_db(
            "CREATE TABLE Tag (TagId INTEGER PRIMARY KEY, Type INTEGER, Name TEXT);
             INSERT INTO Tag (Type, Name) VALUES (2, 'My Playlist');
             INSERT INTO Tag (Type, Name) VALUES (1, 'Other');",
        );
        let file = SqliteFile::parse(&db).unwrap();
        let table = file.read_table("Tag").unwrap();
        assert_eq!(table.rows.len(), 2);
        let row = &table.rows[0];
        assert_eq!(table.get(row, "TagId").unwrap().as_i64(), Some(1));
        assert_eq!(table.get(row, "Type").unwrap().as_i64(), Some(2));
        assert_eq!(
            table.get(row, "Name").unwrap().as_str(),
            Some("My Playlist")
        );
    }

    #[test]
    fn handles_many_rows_and_interior_pages() {
        let mut sql =
            String::from("CREATE TABLE T (Id INTEGER PRIMARY KEY, Label TEXT, N INTEGER);");
        for i in 0..2000 {
            sql.push_str(&format!(
                "INSERT INTO T (Label, N) VALUES ('row {i} with some padding text to fill pages', {i});"
            ));
        }
        let db = make_db(&sql);
        let file = SqliteFile::parse(&db).unwrap();
        let table = file.read_table("T").unwrap();
        assert_eq!(table.rows.len(), 2000);
        let last = table.rows.last().unwrap();
        assert_eq!(table.get(last, "N").unwrap().as_i64(), Some(1999));
        assert_eq!(table.get(last, "Id").unwrap().as_i64(), Some(2000));
    }

    #[test]
    fn handles_overflow_payloads() {
        let big = "x".repeat(50_000);
        let db = make_db(&format!(
            "CREATE TABLE T (Id INTEGER PRIMARY KEY, Data TEXT);
             INSERT INTO T (Data) VALUES ('{big}');"
        ));
        let file = SqliteFile::parse(&db).unwrap();
        let table = file.read_table("T").unwrap();
        let row = &table.rows[0];
        assert_eq!(
            table.get(row, "Data").unwrap().as_str().map(|s| s.len()),
            Some(50_000)
        );
    }

    #[test]
    fn reads_reals_nulls_and_blobs() {
        let db = make_db(
            "CREATE TABLE T (Id INTEGER PRIMARY KEY, F REAL, S TEXT, B BLOB);
             INSERT INTO T (F, S, B) VALUES (1.5, NULL, x'DEADBEEF');",
        );
        let file = SqliteFile::parse(&db).unwrap();
        let table = file.read_table("T").unwrap();
        let row = &table.rows[0];
        assert_eq!(table.get(row, "F"), Some(&Value::Real(1.5)));
        assert_eq!(table.get(row, "S"), Some(&Value::Null));
        assert_eq!(
            table.get(row, "B"),
            Some(&Value::Blob(vec![0xDE, 0xAD, 0xBE, 0xEF]))
        );
    }

    #[test]
    fn reads_without_rowid_table_with_inline_pk() {
        let db = make_db(
            "CREATE TABLE T (Key TEXT PRIMARY KEY, A INTEGER, B TEXT) WITHOUT ROWID;
             INSERT INTO T VALUES ('k2', 2, 'two');
             INSERT INTO T VALUES ('k1', 1, 'one');",
        );
        let file = SqliteFile::parse(&db).unwrap();
        let table = file.read_table("T").unwrap();
        assert_eq!(table.rows.len(), 2);
        // Stored in PK order (k1 first); columns restored to declared order.
        let row = &table.rows[0];
        assert_eq!(table.get(row, "Key").unwrap().as_str(), Some("k1"));
        assert_eq!(table.get(row, "A").unwrap().as_i64(), Some(1));
        assert_eq!(table.get(row, "B").unwrap().as_str(), Some("one"));
    }

    #[test]
    fn reads_without_rowid_table_with_constraint_pk_out_of_order() {
        // PK is (C, A) — not the declared order — so storage is C, A, B, D.
        let db = make_db(
            "CREATE TABLE T (A INTEGER, B TEXT, C INTEGER, D TEXT,
                 PRIMARY KEY (C, A)) WITHOUT ROWID;
             INSERT INTO T VALUES (1, 'b1', 10, 'd1');
             INSERT INTO T VALUES (2, 'b2', 20, 'd2');",
        );
        let file = SqliteFile::parse(&db).unwrap();
        let table = file.read_table("T").unwrap();
        assert_eq!(table.rows.len(), 2);
        let row = &table.rows[0];
        assert_eq!(table.get(row, "A").unwrap().as_i64(), Some(1));
        assert_eq!(table.get(row, "B").unwrap().as_str(), Some("b1"));
        assert_eq!(table.get(row, "C").unwrap().as_i64(), Some(10));
        assert_eq!(table.get(row, "D").unwrap().as_str(), Some("d1"));
    }

    #[test]
    fn reads_large_without_rowid_table_with_interior_pages() {
        let mut sql =
            String::from("CREATE TABLE T (Id INTEGER PRIMARY KEY, Label TEXT) WITHOUT ROWID;");
        for i in 0..2000 {
            sql.push_str(&format!(
                "INSERT INTO T VALUES ({i}, 'row {i} padded with text to fill up pages quickly');"
            ));
        }
        let db = make_db(&sql);
        let file = SqliteFile::parse(&db).unwrap();
        let table = file.read_table("T").unwrap();
        assert_eq!(table.rows.len(), 2000);
        // Every row present exactly once (interior index cells carry rows).
        let mut ids: Vec<i64> = table
            .rows
            .iter()
            .map(|r| table.get(r, "Id").unwrap().as_i64().unwrap())
            .collect();
        ids.sort();
        assert_eq!(ids, (0..2000).collect::<Vec<i64>>());
    }

    #[test]
    fn missing_table_errors() {
        let db = make_db("CREATE TABLE T (Id INTEGER PRIMARY KEY);");
        let file = SqliteFile::parse(&db).unwrap();
        assert!(file.read_table("Nope").is_err());
    }

    #[test]
    fn rejects_non_sqlite_data() {
        assert!(SqliteFile::parse(b"not a database at all").is_err());
    }
}
