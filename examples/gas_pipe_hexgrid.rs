/// Reads a real-world parquet dataset of gas-pipe infrastructure and tags every
/// feature with the list of BNG hex cells its pipe run passes through, at zoom 11.
///
/// The source file (Ordnance-Survey-style open gas-pipe data) stores geometry as
/// WKB BLOB columns in WGS84:
///   * `geo_point_2d` — a representative Point per feature
///   * `geo_shape`    — the actual pipe run, a (Multi)LineString
///
/// We walk each feature's line (`geo_shape`) through the grid with
/// `HexCell::from_geometry(.., Crs::Wgs84, ..)`, which reprojects WGS84 -> BNG
/// with OSTN15 and returns every cell the line touches. The original rows are
/// written back out with one extra column, `hex_ids` (a `List<Utf8>`): the set of
/// hex IDs that feature runs through. So the output parquet is the input plus a
/// per-feature list of hex cells — ready to UNNEST / GROUP BY downstream.
///
/// n3gb-rs is a writer, not a parquet reader, so the example reads the file with
/// the `parquet`/`arrow-array` crates directly and decodes the WKB itself (small
/// reader at the bottom). Per-row indexing is parallelised with rayon.
///
/// Run with (defaults: zoom 11, the path below):
///   cargo run --release --example gas_pipe_hexgrid
///   cargo run --release --example gas_pipe_hexgrid -- 11 /path/to/file.parquet
///                                                      ^zoom ^input path
use std::collections::HashSet;
use std::fs::File;
use std::sync::Arc;
use std::time::Instant;

use arrow_array::builder::{ListBuilder, StringBuilder};
use arrow_array::{Array, ArrayRef, BinaryArray, LargeBinaryArray, RecordBatch, RecordBatchReader};
use arrow_schema::{DataType, Field, Schema};
use parquet::arrow::ArrowWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use rayon::prelude::*;

use geo_types::{
    Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon, coord,
};
use n3gb_rs::{ConversionMethod, Crs, HexCell, N3gbError};

const DEFAULT_INPUT: &str =
    "/Users/cmcarlon/Downloads/data/gas-pipe-infrastructure-gpi_open.parquet";
/// The pipe-run geometry we walk through the grid: a (Multi)LineString.
const GEOMETRY_COLUMN: &str = "geo_shape";
const HEX_IDS_COLUMN: &str = "hex_ids";
const BATCH_SIZE: usize = 16_384;

fn main() -> Result<(), N3gbError> {
    let mut args = std::env::args().skip(1);
    let zoom: u8 = args.next().and_then(|s| s.parse().ok()).unwrap_or(11);
    let input = args.next().unwrap_or_else(|| DEFAULT_INPUT.to_string());
    let method = ConversionMethod::Ostn15;
    let output = format!("gas-pipes-with-hexids-z{}.parquet", zoom);

    println!("=== Gas-pipe infrastructure -> hex ID lists ===");
    println!("  Input:  {}", input);
    println!(
        "  Column: {} (WKB (Multi)LineString, WGS84)",
        GEOMETRY_COLUMN
    );
    println!("  Zoom:   {}", zoom);
    println!("  Output: {} (input + `{}`)\n", output, HEX_IDS_COLUMN);

    let start = Instant::now();

    // --- Open the parquet file (all columns; we pass them through unchanged). ---
    let file = File::open(&input).map_err(|e| N3gbError::IoError(e.to_string()))?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| N3gbError::IoError(e.to_string()))?
        .with_batch_size(BATCH_SIZE)
        .build()
        .map_err(|e| N3gbError::IoError(e.to_string()))?;

    let in_schema = reader.schema();
    let geom_idx = in_schema
        .index_of(GEOMETRY_COLUMN)
        .map_err(|e| N3gbError::IoError(e.to_string()))?;

    // Output schema = input schema + a nullable `hex_ids` List<Utf8> column.
    // The inner field is named "item" / nullable to match what ListBuilder emits.
    let list_type = DataType::List(Arc::new(Field::new("item", DataType::Utf8, true)));
    let mut fields: Vec<Arc<Field>> = in_schema.fields().iter().cloned().collect();
    fields.push(Arc::new(Field::new(HEX_IDS_COLUMN, list_type, true)));
    let out_schema = Arc::new(Schema::new(fields));

    let out_file = File::create(&output).map_err(|e| N3gbError::IoError(e.to_string()))?;
    let mut writer = ArrowWriter::try_new(out_file, out_schema.clone(), None)?;

    let mut features = 0usize;
    let mut memberships = 0usize; // total (feature, cell) pairs written
    let mut failed = 0usize;

    for batch in reader {
        let batch = batch.map_err(|e| N3gbError::IoError(e.to_string()))?;

        // Decode each row's line -> the distinct cells it passes through, in
        // parallel. A missing/unparseable geometry yields a null list.
        let blobs = binary_values(batch.column(geom_idx))?;
        let per_row: Vec<Option<Vec<String>>> = blobs
            .par_iter()
            .map(|maybe| {
                let wkb = (*maybe)?;
                let geom = read_wkb(wkb).ok()?;
                let cells = HexCell::from_geometry(geom, zoom, Crs::Wgs84, method).ok()?;
                // Distinct IDs in first-seen order.
                let mut seen = HashSet::new();
                let ids: Vec<String> = cells
                    .into_iter()
                    .filter(|c| seen.insert(c.id.clone()))
                    .map(|c| c.id)
                    .collect();
                Some(ids)
            })
            .collect();

        // Build the List<Utf8> column, keeping row alignment.
        let mut builder = ListBuilder::new(StringBuilder::new());
        for row in &per_row {
            match row {
                Some(ids) => {
                    for id in ids {
                        builder.values().append_value(id);
                    }
                    builder.append(true);
                    features += 1;
                    memberships += ids.len();
                }
                None => {
                    builder.append(false);
                    features += 1;
                    failed += 1;
                }
            }
        }
        let hex_ids: ArrayRef = Arc::new(builder.finish());

        // Original columns, plus the new hex_ids column, written straight through.
        let mut columns: Vec<ArrayRef> = batch.columns().to_vec();
        columns.push(hex_ids);
        let out_batch = RecordBatch::try_new(out_schema.clone(), columns)?;
        writer.write(&out_batch)?;

        print!("\r  {} features tagged...", features);
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
    writer.close()?;
    println!();

    println!("\n  Features:            {}", features);
    println!("  Cell memberships:    {}", memberships);
    if features > failed && features > 0 {
        println!(
            "  Avg cells/feature:   {:.2}",
            memberships as f64 / (features - failed).max(1) as f64
        );
    }
    if failed > 0 {
        println!(
            "  Null hex_ids:        {} (missing/unparseable geometry)",
            failed
        );
    }
    println!(
        "\n  Wrote {} -> input columns + `{}`",
        output, HEX_IDS_COLUMN
    );

    let elapsed = start.elapsed();
    let rate = features as f64 / elapsed.as_secs_f64();
    println!(
        "\n  Elapsed: {:.2}s ({:.0} features/sec)",
        elapsed.as_secs_f64(),
        rate
    );

    Ok(())
}

/// Borrow the values of a Binary / LargeBinary Arrow column, keeping row
/// alignment: index `i` is `None` if that row's geometry is null.
fn binary_values(column: &dyn Array) -> Result<Vec<Option<&[u8]>>, N3gbError> {
    if let Some(a) = column.as_any().downcast_ref::<BinaryArray>() {
        Ok((0..a.len())
            .map(|i| a.is_valid(i).then(|| a.value(i)))
            .collect())
    } else if let Some(a) = column.as_any().downcast_ref::<LargeBinaryArray>() {
        Ok((0..a.len())
            .map(|i| a.is_valid(i).then(|| a.value(i)))
            .collect())
    } else {
        Err(N3gbError::GeometryParseError(format!(
            "column `{}` is not a binary/WKB column ({:?})",
            GEOMETRY_COLUMN,
            column.data_type()
        )))
    }
}

// ---------------------------------------------------------------------------
// Minimal ISO-WKB reader (2D). Enough to decode the Point and (Multi)LineString
// geometries in this dataset; also handles polygons / collections for safety.
// ---------------------------------------------------------------------------

struct WkbCursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> WkbCursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.pos + n;
        if end > self.buf.len() {
            return Err("unexpected end of WKB".to_string());
        }
        let slice = &self.buf[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self, le: bool) -> Result<u32, String> {
        let b: [u8; 4] = self.take(4)?.try_into().unwrap();
        Ok(if le {
            u32::from_le_bytes(b)
        } else {
            u32::from_be_bytes(b)
        })
    }

    fn f64(&mut self, le: bool) -> Result<f64, String> {
        let b: [u8; 8] = self.take(8)?.try_into().unwrap();
        Ok(if le {
            f64::from_le_bytes(b)
        } else {
            f64::from_be_bytes(b)
        })
    }

    fn coord(&mut self, le: bool) -> Result<geo_types::Coord<f64>, String> {
        let x = self.f64(le)?;
        let y = self.f64(le)?;
        Ok(coord! { x: x, y: y })
    }

    fn line_string(&mut self, le: bool) -> Result<LineString<f64>, String> {
        let n = self.u32(le)? as usize;
        let mut coords = Vec::with_capacity(n);
        for _ in 0..n {
            coords.push(self.coord(le)?);
        }
        Ok(LineString::new(coords))
    }

    fn polygon(&mut self, le: bool) -> Result<Polygon<f64>, String> {
        let rings = self.u32(le)? as usize;
        if rings == 0 {
            return Ok(Polygon::new(LineString::new(vec![]), vec![]));
        }
        let exterior = self.line_string(le)?;
        let mut interiors = Vec::with_capacity(rings - 1);
        for _ in 1..rings {
            interiors.push(self.line_string(le)?);
        }
        Ok(Polygon::new(exterior, interiors))
    }

    /// Read one full geometry (its own byte-order + type header).
    fn geometry(&mut self) -> Result<Geometry<f64>, String> {
        let le = match self.u8()? {
            1 => true,
            0 => false,
            other => return Err(format!("invalid WKB byte order: {}", other)),
        };
        // Low byte carries the base 2D type; higher bytes (Z/M/SRID flags) are
        // ignored — this dataset is plain 2D.
        let geom_type = self.u32(le)? & 0xff;
        match geom_type {
            1 => Ok(Geometry::Point(Point::from(self.coord(le)?))),
            2 => Ok(Geometry::LineString(self.line_string(le)?)),
            3 => Ok(Geometry::Polygon(self.polygon(le)?)),
            4 => {
                let n = self.u32(le)? as usize;
                let mut pts = Vec::with_capacity(n);
                for _ in 0..n {
                    match self.geometry()? {
                        Geometry::Point(p) => pts.push(p),
                        _ => return Err("MultiPoint contained non-Point".to_string()),
                    }
                }
                Ok(Geometry::MultiPoint(MultiPoint::new(pts)))
            }
            5 => {
                let n = self.u32(le)? as usize;
                let mut lines = Vec::with_capacity(n);
                for _ in 0..n {
                    match self.geometry()? {
                        Geometry::LineString(l) => lines.push(l),
                        _ => return Err("MultiLineString contained non-LineString".to_string()),
                    }
                }
                Ok(Geometry::MultiLineString(MultiLineString::new(lines)))
            }
            6 => {
                let n = self.u32(le)? as usize;
                let mut polys = Vec::with_capacity(n);
                for _ in 0..n {
                    match self.geometry()? {
                        Geometry::Polygon(p) => polys.push(p),
                        _ => return Err("MultiPolygon contained non-Polygon".to_string()),
                    }
                }
                Ok(Geometry::MultiPolygon(MultiPolygon::new(polys)))
            }
            7 => {
                let n = self.u32(le)? as usize;
                let mut geoms = Vec::with_capacity(n);
                for _ in 0..n {
                    geoms.push(self.geometry()?);
                }
                Ok(Geometry::GeometryCollection(geoms.into()))
            }
            other => Err(format!("unsupported WKB geometry type: {}", other)),
        }
    }
}

fn read_wkb(buf: &[u8]) -> Result<Geometry<f64>, String> {
    let mut cursor = WkbCursor { buf, pos: 0 };
    cursor.geometry()
}
