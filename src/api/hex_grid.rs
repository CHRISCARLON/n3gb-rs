use crate::api::hex_arrow::HexCellsToArrow;
use crate::api::hex_cell::HexCell;
use crate::api::hex_parquet::HexCellsToGeoParquet;
use crate::core::constants::{GRID_EXTENTS, MAX_ZOOM_LEVEL};
use crate::core::grid::{hex_to_point, point_to_hex};
use crate::util::coord::{
    wgs84_multipolygon_to_bng, wgs84_polygon_to_bng, wgs84_to_bng, Coordinate,
};
use crate::util::error::N3gbError;
use crate::util::identifier::generate_identifier;
use arrow_array::RecordBatch;
use geo::{BoundingRect, Intersects};
use geo_types::{MultiPolygon, Point, Polygon, Rect};
use geoarrow_array::array::{PointArray, PolygonArray};
use rayon::prelude::*;
use std::path::Path;

/// A collection of hexagonal cells covering a geographic extent.
///
/// `HexGrid` generates and manages multiple [`HexCell`]s for a given bounding box
/// and zoom level. Use it when you need to work with multiple cells at once,
/// such as tiling an area prior to performing spatial queries.
///
/// # Example
///
/// ```
/// use n3gb_rs::HexGrid;
/// use geo_types::point;
///
/// // Create a grid covering an area
/// let grid = HexGrid::builder()
///     .zoom_level(10)
///     .bng_extent(&(457000.0, 339500.0), &(458000.0, 340500.0))
///     .build();
///
/// println!("Grid contains {} cells", grid.len());
///
/// // Find which cell contains a point
/// let pt = point! { x: 457500.0, y: 340000.0 };
/// if let Some(cell) = grid.get_cell_at(&pt) {
///     println!("Point is in cell: {}", cell.id);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct HexGrid {
    cells: Vec<HexCell>,
    zoom_level: u8,
}

impl HexGrid {
    /// Creates a new [`HexGridBuilder`] for grid construction.
    pub fn builder() -> HexGridBuilder {
        HexGridBuilder::new()
    }

    fn from_extent(min_x: f64, min_y: f64, max_x: f64, max_y: f64, zoom_level: u8) -> Self {
        let cells = generate_cells_for_extent(min_x, min_y, max_x, max_y, zoom_level);
        Self { cells, zoom_level }
    }

    /// Creates a HexGrid from a `geo_types::Rect` in BNG coordinates.
    pub fn from_rect(rect: &Rect<f64>, zoom_level: u8) -> Self {
        Self::from_extent(
            rect.min().x,
            rect.min().y,
            rect.max().x,
            rect.max().y,
            zoom_level,
        )
    }

    /// Create a HexGrid from British National Grid coordinates
    ///
    /// # Example
    /// ```
    /// use n3gb_rs::HexGrid;
    /// use geo_types::Point;
    ///
    /// // From tuples
    /// let grid = HexGrid::from_bng_extent(&(457000.0, 339500.0), &(458000.0, 340500.0), 10);
    /// // From Points
    /// let grid = HexGrid::from_bng_extent(
    ///     &Point::new(457000.0, 339500.0),
    ///     &Point::new(458000.0, 340500.0),
    ///     10
    /// );
    /// ```
    pub fn from_bng_extent(min: &impl Coordinate, max: &impl Coordinate, zoom_level: u8) -> Self {
        Self::from_extent(min.x(), min.y(), max.x(), max.y(), zoom_level)
    }

    /// Create a HexGrid from WGS84 (lon/lat) coordinates
    ///
    /// # Example
    /// ```
    /// use n3gb_rs::HexGrid;
    /// use geo_types::Point;
    ///
    /// # fn main() -> Result<(), n3gb_rs::N3gbError> {
    /// // From tuples (lon, lat)
    /// let grid = HexGrid::from_wgs84_extent(&(-2.3, 53.4), &(-2.2, 53.5), 10)?;
    /// // From Points
    /// let grid = HexGrid::from_wgs84_extent(
    ///     &Point::new(-2.3, 53.4),
    ///     &Point::new(-2.2, 53.5),
    ///     10
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_wgs84_extent(
        min: &impl Coordinate,
        max: &impl Coordinate,
        zoom_level: u8,
    ) -> Result<Self, N3gbError> {
        let min_bng = wgs84_to_bng(min)?;
        let max_bng = wgs84_to_bng(max)?;
        Ok(Self::from_extent(
            min_bng.x(),
            min_bng.y(),
            max_bng.x(),
            max_bng.y(),
            zoom_level,
        ))
    }

    /// Creates a HexGrid from a polygon in BNG coordinates.
    ///
    /// Generates hex cells for the polygon's bounding box, then filters
    /// to only include cells whose hexagon intersects the polygon.
    ///
    /// # Example
    /// ```
    /// use n3gb_rs::HexGrid;
    /// use geo_types::{Polygon, LineString, coord};
    ///
    /// let polygon = Polygon::new(
    ///     LineString::from(vec![
    ///         coord! { x: 457000.0, y: 339500.0 },
    ///         coord! { x: 458000.0, y: 339500.0 },
    ///         coord! { x: 458000.0, y: 340500.0 },
    ///         coord! { x: 457000.0, y: 340500.0 },
    ///         coord! { x: 457000.0, y: 339500.0 },
    ///     ]),
    ///     vec![],
    /// );
    /// let grid = HexGrid::from_bng_polygon(&polygon, 10);
    /// assert!(!grid.is_empty());
    /// ```
    pub fn from_bng_polygon(polygon: &Polygon<f64>, zoom_level: u8) -> Self {
        let bbox = match polygon.bounding_rect() {
            Some(rect) => rect,
            None => {
                return Self {
                    cells: Vec::new(),
                    zoom_level,
                };
            }
        };

        let candidate_cells = generate_cells_for_extent(
            bbox.min().x,
            bbox.min().y,
            bbox.max().x,
            bbox.max().y,
            zoom_level,
        );

        let cells: Vec<HexCell> = candidate_cells
            .into_par_iter()
            .filter(|cell| polygon.intersects(&cell.to_polygon()))
            .collect();

        Self { cells, zoom_level }
    }

    /// Creates a HexGrid from a polygon in WGS84 (lon/lat) coordinates.
    ///
    /// Projects the polygon to BNG, then generates hex cells for the
    /// polygon's bounding box and filters to cells that intersect.
    ///
    /// # Example
    /// ```
    /// use n3gb_rs::HexGrid;
    /// use geo_types::{Polygon, LineString, coord};
    ///
    /// # fn main() -> Result<(), n3gb_rs::N3gbError> {
    /// let polygon = Polygon::new(
    ///     LineString::from(vec![
    ///         coord! { x: -2.3, y: 53.4 },
    ///         coord! { x: -2.2, y: 53.4 },
    ///         coord! { x: -2.2, y: 53.5 },
    ///         coord! { x: -2.3, y: 53.5 },
    ///         coord! { x: -2.3, y: 53.4 },
    ///     ]),
    ///     vec![],
    /// );
    /// let grid = HexGrid::from_wgs84_polygon(&polygon, 10)?;
    /// assert!(!grid.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_wgs84_polygon(polygon: &Polygon<f64>, zoom_level: u8) -> Result<Self, N3gbError> {
        let bng_polygon = wgs84_polygon_to_bng(polygon)?;
        Ok(Self::from_bng_polygon(&bng_polygon, zoom_level))
    }

    /// Creates a HexGrid from a multipolygon in BNG coordinates.
    ///
    /// Generates hex cells for each polygon in the multipolygon and
    /// combines them, deduplicating overlapping cells.
    ///
    /// # Example
    /// ```
    /// use n3gb_rs::HexGrid;
    /// use geo_types::{MultiPolygon, Polygon, LineString, coord};
    ///
    /// let poly1 = Polygon::new(
    ///     LineString::from(vec![
    ///         coord! { x: 457000.0, y: 339500.0 },
    ///         coord! { x: 457500.0, y: 339500.0 },
    ///         coord! { x: 457500.0, y: 340000.0 },
    ///         coord! { x: 457000.0, y: 340000.0 },
    ///         coord! { x: 457000.0, y: 339500.0 },
    ///     ]),
    ///     vec![],
    /// );
    /// let poly2 = Polygon::new(
    ///     LineString::from(vec![
    ///         coord! { x: 457500.0, y: 340000.0 },
    ///         coord! { x: 458000.0, y: 340000.0 },
    ///         coord! { x: 458000.0, y: 340500.0 },
    ///         coord! { x: 457500.0, y: 340500.0 },
    ///         coord! { x: 457500.0, y: 340000.0 },
    ///     ]),
    ///     vec![],
    /// );
    /// let mp = MultiPolygon::new(vec![poly1, poly2]);
    /// let grid = HexGrid::from_bng_multipolygon(&mp, 10);
    /// assert!(!grid.is_empty());
    /// ```
    pub fn from_bng_multipolygon(multipolygon: &MultiPolygon<f64>, zoom_level: u8) -> Self {
        use std::collections::HashSet;

        let mut seen_ids = HashSet::new();
        let cells: Vec<HexCell> = multipolygon
            .0
            .par_iter()
            .flat_map(|polygon| Self::from_bng_polygon(polygon, zoom_level).cells)
            .collect::<Vec<_>>()
            .into_iter()
            .filter(|cell| seen_ids.insert(cell.id.clone()))
            .collect();

        Self { cells, zoom_level }
    }

    /// Creates a HexGrid from a multipolygon in WGS84 (lon/lat) coordinates.
    ///
    /// Projects the multipolygon to BNG, then generates hex cells for each
    /// polygon and combines them, deduplicating overlapping cells.
    ///
    /// # Example
    /// ```
    /// use n3gb_rs::HexGrid;
    /// use geo_types::{MultiPolygon, Polygon, LineString, coord};
    ///
    /// # fn main() -> Result<(), n3gb_rs::N3gbError> {
    /// let poly1 = Polygon::new(
    ///     LineString::from(vec![
    ///         coord! { x: -2.3, y: 53.4 },
    ///         coord! { x: -2.25, y: 53.4 },
    ///         coord! { x: -2.25, y: 53.45 },
    ///         coord! { x: -2.3, y: 53.45 },
    ///         coord! { x: -2.3, y: 53.4 },
    ///     ]),
    ///     vec![],
    /// );
    /// let poly2 = Polygon::new(
    ///     LineString::from(vec![
    ///         coord! { x: -2.25, y: 53.45 },
    ///         coord! { x: -2.2, y: 53.45 },
    ///         coord! { x: -2.2, y: 53.5 },
    ///         coord! { x: -2.25, y: 53.5 },
    ///         coord! { x: -2.25, y: 53.45 },
    ///     ]),
    ///     vec![],
    /// );
    /// let mp = MultiPolygon::new(vec![poly1, poly2]);
    /// let grid = HexGrid::from_wgs84_multipolygon(&mp, 10)?;
    /// assert!(!grid.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_wgs84_multipolygon(
        multipolygon: &MultiPolygon<f64>,
        zoom_level: u8,
    ) -> Result<Self, N3gbError> {
        let bng_multipolygon = wgs84_multipolygon_to_bng(multipolygon)?;
        Ok(Self::from_bng_multipolygon(&bng_multipolygon, zoom_level))
    }

    /// Returns the zoom level of this grid.
    pub fn zoom_level(&self) -> u8 {
        self.zoom_level
    }

    /// Returns the number of cells in this grid.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Returns `true` if the grid contains no cells.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Returns a slice of all cells in this grid.
    pub fn cells(&self) -> &[HexCell] {
        &self.cells
    }

    /// Returns an iterator over the cells in this grid.
    pub fn iter(&self) -> impl Iterator<Item = &HexCell> {
        self.cells.iter()
    }

    /// Finds the cell containing the given point, if any.
    ///
    /// Returns `None` if the point is outside the grid's extent.
    pub fn get_cell_at(&self, point: &Point<f64>) -> Option<&HexCell> {
        let (row, col) = point_to_hex(point, self.zoom_level).ok()?;
        self.cells
            .iter()
            .find(|cell| cell.row == row && cell.col == col)
    }

    /// Converts all cells to hexagonal polygons.
    pub fn to_polygons(&self) -> Vec<Polygon<f64>> {
        self.cells
            .par_iter()
            .map(|cell| cell.to_polygon())
            .collect()
    }

    /// Returns cells matching the given predicate.
    pub fn filter<F>(&self, predicate: F) -> Vec<&HexCell>
    where
        F: Fn(&HexCell) -> bool,
    {
        self.cells.iter().filter(|cell| predicate(cell)).collect()
    }

    /// Converts all cell centers to an Arrow PointArray.
    pub fn to_arrow_points(&self) -> PointArray {
        self.cells.to_arrow_points()
    }

    /// Converts all cells to an Arrow PolygonArray.
    pub fn to_arrow_polygons(&self) -> PolygonArray {
        self.cells.to_arrow_polygons()
    }

    /// Converts all cells to an Arrow RecordBatch with all attributes.
    pub fn to_record_batch(&self) -> Result<RecordBatch, N3gbError> {
        self.cells.to_record_batch()
    }

    /// Writes all cells to a GeoParquet file.
    pub fn to_geoparquet(&self, path: impl AsRef<Path>) -> Result<(), N3gbError> {
        self.cells.to_geoparquet(path)
    }
}

/// Builder for constructing a [`HexGrid`] with a fluent API.
///
/// # Example
///
/// ```
/// use n3gb_rs::HexGrid;
///
/// let grid = HexGrid::builder()
///     .zoom_level(10)
///     .bng_extent(&(457000.0, 339500.0), &(458000.0, 340500.0))
///     .build();
/// ```
#[derive(Debug, Default, Clone)]
pub struct HexGridBuilder {
    zoom_level: Option<u8>,
    min_x: Option<f64>,
    min_y: Option<f64>,
    max_x: Option<f64>,
    max_y: Option<f64>,
    polygon: Option<Polygon<f64>>,
    multipolygon: Option<MultiPolygon<f64>>,
}

impl HexGridBuilder {
    /// Creates a new builder with no parameters set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the zoom level (0-15).
    pub fn zoom_level(mut self, zoom_level: u8) -> Self {
        self.zoom_level = Some(zoom_level);
        self
    }

    /// Sets the extent from a `geo_types::Rect` in BNG coordinates.
    pub fn rect(mut self, rect: &Rect<f64>) -> Self {
        self.min_x = Some(rect.min().x);
        self.min_y = Some(rect.min().y);
        self.max_x = Some(rect.max().x);
        self.max_y = Some(rect.max().y);
        self
    }

    /// Set extent from British National Grid coordinates
    ///
    /// # Example
    /// ```
    /// use n3gb_rs::HexGrid;
    ///
    /// let grid = HexGrid::builder()
    ///     .zoom_level(10)
    ///     .bng_extent(&(457000.0, 339500.0), &(458000.0, 340500.0))
    ///     .build();
    /// ```
    pub fn bng_extent(mut self, min: &impl Coordinate, max: &impl Coordinate) -> Self {
        self.min_x = Some(min.x());
        self.min_y = Some(min.y());
        self.max_x = Some(max.x());
        self.max_y = Some(max.y());
        self
    }

    /// Set extent from WGS84 (lon/lat) coordinates
    ///
    /// # Example
    /// ```
    /// use n3gb_rs::HexGrid;
    ///
    /// # fn main() -> Result<(), n3gb_rs::N3gbError> {
    /// let grid = HexGrid::builder()
    ///     .zoom_level(10)
    ///     .wgs84_extent(&(-2.3, 53.4), &(-2.2, 53.5))?
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    pub fn wgs84_extent(
        mut self,
        min: &impl Coordinate,
        max: &impl Coordinate,
    ) -> Result<Self, N3gbError> {
        let min_bng = wgs84_to_bng(min)?;
        let max_bng = wgs84_to_bng(max)?;
        self.min_x = Some(min_bng.x());
        self.min_y = Some(min_bng.y());
        self.max_x = Some(max_bng.x());
        self.max_y = Some(max_bng.y());
        Ok(self)
    }

    /// Sets the geometry from a polygon in BNG coordinates.
    ///
    /// When a polygon is set, the grid will only include cells that
    /// intersect the polygon, not the full bounding box.
    ///
    /// # Example
    /// ```
    /// use n3gb_rs::HexGrid;
    /// use geo_types::{Polygon, LineString, coord};
    ///
    /// let polygon = Polygon::new(
    ///     LineString::from(vec![
    ///         coord! { x: 457000.0, y: 339500.0 },
    ///         coord! { x: 458000.0, y: 339500.0 },
    ///         coord! { x: 458000.0, y: 340500.0 },
    ///         coord! { x: 457000.0, y: 340500.0 },
    ///         coord! { x: 457000.0, y: 339500.0 },
    ///     ]),
    ///     vec![],
    /// );
    /// let grid = HexGrid::builder()
    ///     .zoom_level(10)
    ///     .bng_polygon(polygon)
    ///     .build();
    /// ```
    pub fn bng_polygon(mut self, polygon: Polygon<f64>) -> Self {
        self.polygon = Some(polygon);
        self
    }

    /// Sets the geometry from a polygon in WGS84 (lon/lat) coordinates.
    ///
    /// Projects the polygon to BNG, then filters cells to those
    /// that intersect the polygon.
    ///
    /// # Example
    /// ```
    /// use n3gb_rs::HexGrid;
    /// use geo_types::{Polygon, LineString, coord};
    ///
    /// # fn main() -> Result<(), n3gb_rs::N3gbError> {
    /// let polygon = Polygon::new(
    ///     LineString::from(vec![
    ///         coord! { x: -2.3, y: 53.4 },
    ///         coord! { x: -2.2, y: 53.4 },
    ///         coord! { x: -2.2, y: 53.5 },
    ///         coord! { x: -2.3, y: 53.5 },
    ///         coord! { x: -2.3, y: 53.4 },
    ///     ]),
    ///     vec![],
    /// );
    /// let grid = HexGrid::builder()
    ///     .zoom_level(10)
    ///     .wgs84_polygon(polygon)?
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    pub fn wgs84_polygon(mut self, polygon: Polygon<f64>) -> Result<Self, N3gbError> {
        let bng_polygon = wgs84_polygon_to_bng(&polygon)?;
        self.polygon = Some(bng_polygon);
        Ok(self)
    }

    /// Sets the geometry from a multipolygon in BNG coordinates.
    ///
    /// When a multipolygon is set, the grid will only include cells that
    /// intersect any of the polygons, with duplicates removed.
    ///
    /// # Example
    /// ```
    /// use n3gb_rs::HexGrid;
    /// use geo_types::{MultiPolygon, Polygon, LineString, coord};
    ///
    /// let poly1 = Polygon::new(
    ///     LineString::from(vec![
    ///         coord! { x: 457000.0, y: 339500.0 },
    ///         coord! { x: 457500.0, y: 339500.0 },
    ///         coord! { x: 457500.0, y: 340000.0 },
    ///         coord! { x: 457000.0, y: 340000.0 },
    ///         coord! { x: 457000.0, y: 339500.0 },
    ///     ]),
    ///     vec![],
    /// );
    /// let poly2 = Polygon::new(
    ///     LineString::from(vec![
    ///         coord! { x: 457500.0, y: 340000.0 },
    ///         coord! { x: 458000.0, y: 340000.0 },
    ///         coord! { x: 458000.0, y: 340500.0 },
    ///         coord! { x: 457500.0, y: 340500.0 },
    ///         coord! { x: 457500.0, y: 340000.0 },
    ///     ]),
    ///     vec![],
    /// );
    /// let mp = MultiPolygon::new(vec![poly1, poly2]);
    /// let grid = HexGrid::builder()
    ///     .zoom_level(10)
    ///     .bng_multipolygon(mp)
    ///     .build();
    /// ```
    pub fn bng_multipolygon(mut self, multipolygon: MultiPolygon<f64>) -> Self {
        self.multipolygon = Some(multipolygon);
        self
    }

    /// Sets the geometry from a multipolygon in WGS84 (lon/lat) coordinates.
    ///
    /// Projects the multipolygon to BNG, then filters cells to those
    /// that intersect any of the polygons.
    ///
    /// # Example
    /// ```
    /// use n3gb_rs::HexGrid;
    /// use geo_types::{MultiPolygon, Polygon, LineString, coord};
    ///
    /// # fn main() -> Result<(), n3gb_rs::N3gbError> {
    /// let poly1 = Polygon::new(
    ///     LineString::from(vec![
    ///         coord! { x: -2.3, y: 53.4 },
    ///         coord! { x: -2.25, y: 53.4 },
    ///         coord! { x: -2.25, y: 53.45 },
    ///         coord! { x: -2.3, y: 53.45 },
    ///         coord! { x: -2.3, y: 53.4 },
    ///     ]),
    ///     vec![],
    /// );
    /// let poly2 = Polygon::new(
    ///     LineString::from(vec![
    ///         coord! { x: -2.25, y: 53.45 },
    ///         coord! { x: -2.2, y: 53.45 },
    ///         coord! { x: -2.2, y: 53.5 },
    ///         coord! { x: -2.25, y: 53.5 },
    ///         coord! { x: -2.25, y: 53.45 },
    ///     ]),
    ///     vec![],
    /// );
    /// let mp = MultiPolygon::new(vec![poly1, poly2]);
    /// let grid = HexGrid::builder()
    ///     .zoom_level(10)
    ///     .wgs84_multipolygon(mp)?
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    pub fn wgs84_multipolygon(
        mut self,
        multipolygon: MultiPolygon<f64>,
    ) -> Result<Self, N3gbError> {
        let bng_multipolygon = wgs84_multipolygon_to_bng(&multipolygon)?;
        self.multipolygon = Some(bng_multipolygon);
        Ok(self)
    }

    /// Builds the [`HexGrid`].
    ///
    /// # Panics
    ///
    /// Panics if `zoom_level` has not been set, or if neither extent, polygon,
    /// nor multipolygon has been set.
    pub fn build(self) -> HexGrid {
        let zoom_level = self.zoom_level.expect("zoom_level must be set");

        match (self.multipolygon, self.polygon) {
            (Some(mp), _) => HexGrid::from_bng_multipolygon(&mp, zoom_level),
            (_, Some(p)) => HexGrid::from_bng_polygon(&p, zoom_level),
            (None, None) => {
                let min_x = self.min_x.expect("extent, polygon, or multipolygon must be set");
                let min_y = self.min_y.expect("extent, polygon, or multipolygon must be set");
                let max_x = self.max_x.expect("extent, polygon, or multipolygon must be set");
                let max_y = self.max_y.expect("extent, polygon, or multipolygon must be set");
                HexGrid::from_extent(min_x, min_y, max_x, max_y, zoom_level)
            }
        }
    }
}

fn generate_cells_for_extent(
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    zoom_level: u8,
) -> Vec<HexCell> {
    if zoom_level > MAX_ZOOM_LEVEL {
        return Vec::new();
    }

    let (ll_row, ll_col) = match point_to_hex(&(min_x, min_y), zoom_level) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let (lr_row, lr_col) = match point_to_hex(&(max_x, min_y), zoom_level) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let (ur_row, ur_col) = match point_to_hex(&(max_x, max_y), zoom_level) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let (ul_row, ul_col) = match point_to_hex(&(min_x, max_y), zoom_level) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let min_row = ll_row.min(lr_row).min(ur_row).min(ul_row);
    let max_row = ll_row.max(lr_row).max(ur_row).max(ul_row);
    let min_col = ll_col.min(lr_col).min(ur_col).min(ul_col);
    let max_col = ll_col.max(lr_col).max(ur_col).max(ul_col);

    (min_row..=max_row)
        .flat_map(|row| (min_col..=max_col).map(move |col| (row, col)))
        .collect::<Vec<_>>()
        .into_par_iter()
        .filter_map(|(row, col)| {
            let center = hex_to_point(row, col, zoom_level).ok()?;

            if center.x() < GRID_EXTENTS[0] || center.y() < GRID_EXTENTS[1] {
                return None;
            }

            let id = generate_identifier(center.x(), center.y(), zoom_level);
            Some(HexCell::new(id, center, zoom_level, row, col))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo_types::{coord, point};

    #[test]
    fn test_hex_grid_from_bng_extent() {
        let grid = HexGrid::from_bng_extent(&(457000.0, 339500.0), &(458000.0, 340500.0), 10);
        assert!(!grid.is_empty());
        assert_eq!(grid.zoom_level(), 10);

        for cell in grid.iter() {
            assert_eq!(cell.zoom_level, 10);
        }
    }

    #[test]
    fn test_hex_grid_from_rect() {
        let rect = Rect::new(
            coord! { x: 457000.0, y: 339500.0 },
            coord! { x: 458000.0, y: 340500.0 },
        );
        let grid = HexGrid::from_rect(&rect, 10);
        assert!(!grid.is_empty());
    }

    #[test]
    fn test_hex_grid_builder() {
        let grid = HexGrid::builder()
            .zoom_level(10)
            .bng_extent(&(457000.0, 339500.0), &(458000.0, 340500.0))
            .build();

        assert!(!grid.is_empty());
        assert_eq!(grid.zoom_level(), 10);
    }

    #[test]
    fn test_hex_grid_builder_with_rect() {
        let rect = Rect::new(
            coord! { x: 457000.0, y: 339500.0 },
            coord! { x: 458000.0, y: 340500.0 },
        );
        let grid = HexGrid::builder().zoom_level(10).rect(&rect).build();

        assert!(!grid.is_empty());
    }

    #[test]
    fn test_get_cell_at() {
        let grid = HexGrid::from_bng_extent(&(457000.0, 339500.0), &(458000.0, 340500.0), 10);
        let pt = point! { x: 457500.0, y: 340000.0 };

        let cell = grid.get_cell_at(&pt);
        assert!(cell.is_some());
    }

    #[test]
    fn test_filter_cells() {
        let grid = HexGrid::from_bng_extent(&(457000.0, 339500.0), &(458000.0, 340500.0), 10);

        let filtered = grid.filter(|cell| cell.easting() > 457500.0);
        assert!(!filtered.is_empty());
    }

    #[test]
    fn test_to_polygons() {
        let grid = HexGrid::from_bng_extent(&(457000.0, 339500.0), &(458000.0, 340500.0), 10);
        let polygons = grid.to_polygons();

        assert_eq!(polygons.len(), grid.len());
    }

    #[test]
    fn test_from_bng_extent_tuple() {
        let grid = HexGrid::from_bng_extent(&(457000.0, 339500.0), &(458000.0, 340500.0), 10);
        assert!(!grid.is_empty());
        assert_eq!(grid.zoom_level(), 10);
    }

    #[test]
    fn test_from_bng_extent_point() {
        let grid = HexGrid::from_bng_extent(
            &point! { x: 457000.0, y: 339500.0 },
            &point! { x: 458000.0, y: 340500.0 },
            10,
        );
        assert!(!grid.is_empty());
        assert_eq!(grid.zoom_level(), 10);
    }

    #[test]
    fn test_from_wgs84_extent_tuple() -> Result<(), N3gbError> {
        let grid = HexGrid::from_wgs84_extent(&(-2.3, 53.4), &(-2.2, 53.5), 10)?;
        assert!(!grid.is_empty());
        assert_eq!(grid.zoom_level(), 10);

        for cell in grid.iter() {
            assert_eq!(cell.zoom_level, 10);
        }
        Ok(())
    }

    #[test]
    fn test_from_wgs84_extent_point() -> Result<(), N3gbError> {
        let grid = HexGrid::from_wgs84_extent(
            &point! { x: -2.3, y: 53.4 },
            &point! { x: -2.2, y: 53.5 },
            10,
        )?;
        assert!(!grid.is_empty());
        Ok(())
    }

    #[test]
    fn test_builder_bng_extent() {
        let grid = HexGrid::builder()
            .zoom_level(10)
            .bng_extent(&(457000.0, 339500.0), &(458000.0, 340500.0))
            .build();

        assert!(!grid.is_empty());
        assert_eq!(grid.zoom_level(), 10);
    }

    #[test]
    fn test_builder_wgs84_extent() -> Result<(), N3gbError> {
        let grid = HexGrid::builder()
            .zoom_level(10)
            .wgs84_extent(&(-2.3, 53.4), &(-2.2, 53.5))?
            .build();

        assert!(!grid.is_empty());
        assert_eq!(grid.zoom_level(), 10);
        Ok(())
    }

    #[test]
    fn test_bng_and_wgs84_same_area() -> Result<(), N3gbError> {
        let bng_grid = HexGrid::from_bng_extent(&(383000.0, 383000.0), &(384000.0, 384000.0), 10);
        let wgs84_grid = HexGrid::from_wgs84_extent(&(-2.26, 53.39), &(-2.24, 53.40), 10)?;
        assert!(!bng_grid.is_empty());
        assert!(!wgs84_grid.is_empty());
        Ok(())
    }

    #[test]
    fn test_from_bng_polygon() {
        use geo_types::LineString;

        let polygon = Polygon::new(
            LineString::from(vec![
                coord! { x: 457000.0, y: 339500.0 },
                coord! { x: 458000.0, y: 339500.0 },
                coord! { x: 458000.0, y: 340500.0 },
                coord! { x: 457000.0, y: 340500.0 },
                coord! { x: 457000.0, y: 339500.0 },
            ]),
            vec![],
        );
        let grid = HexGrid::from_bng_polygon(&polygon, 10);
        assert!(!grid.is_empty());
        assert_eq!(grid.zoom_level(), 10);
    }

    #[test]
    fn test_from_bng_polygon_filters_cells() {
        use geo_types::LineString;

        let triangle = Polygon::new(
            LineString::from(vec![
                coord! { x: 457000.0, y: 339500.0 },
                coord! { x: 458000.0, y: 339500.0 },
                coord! { x: 457500.0, y: 340500.0 },
                coord! { x: 457000.0, y: 339500.0 },
            ]),
            vec![],
        );

        let polygon_grid = HexGrid::from_bng_polygon(&triangle, 10);
        let bbox_grid = HexGrid::from_bng_extent(&(457000.0, 339500.0), &(458000.0, 340500.0), 10);

        assert!(polygon_grid.len() < bbox_grid.len());
        assert!(!polygon_grid.is_empty());
    }

    #[test]
    fn test_from_wgs84_polygon() -> Result<(), N3gbError> {
        use geo_types::LineString;

        let polygon = Polygon::new(
            LineString::from(vec![
                coord! { x: -2.3, y: 53.4 },
                coord! { x: -2.2, y: 53.4 },
                coord! { x: -2.2, y: 53.5 },
                coord! { x: -2.3, y: 53.5 },
                coord! { x: -2.3, y: 53.4 },
            ]),
            vec![],
        );
        let grid = HexGrid::from_wgs84_polygon(&polygon, 10)?;
        assert!(!grid.is_empty());
        assert_eq!(grid.zoom_level(), 10);
        Ok(())
    }

    #[test]
    fn test_builder_bng_polygon() {
        use geo_types::LineString;

        let polygon = Polygon::new(
            LineString::from(vec![
                coord! { x: 457000.0, y: 339500.0 },
                coord! { x: 458000.0, y: 339500.0 },
                coord! { x: 458000.0, y: 340500.0 },
                coord! { x: 457000.0, y: 340500.0 },
                coord! { x: 457000.0, y: 339500.0 },
            ]),
            vec![],
        );
        let grid = HexGrid::builder()
            .zoom_level(10)
            .bng_polygon(polygon)
            .build();

        assert!(!grid.is_empty());
        assert_eq!(grid.zoom_level(), 10);
    }

    #[test]
    fn test_builder_wgs84_polygon() -> Result<(), N3gbError> {
        use geo_types::LineString;

        let polygon = Polygon::new(
            LineString::from(vec![
                coord! { x: -2.3, y: 53.4 },
                coord! { x: -2.2, y: 53.4 },
                coord! { x: -2.2, y: 53.5 },
                coord! { x: -2.3, y: 53.5 },
                coord! { x: -2.3, y: 53.4 },
            ]),
            vec![],
        );
        let grid = HexGrid::builder()
            .zoom_level(10)
            .wgs84_polygon(polygon)?
            .build();

        assert!(!grid.is_empty());
        assert_eq!(grid.zoom_level(), 10);
        Ok(())
    }

    #[test]
    fn test_from_bng_multipolygon() {
        use geo_types::LineString;

        let poly1 = Polygon::new(
            LineString::from(vec![
                coord! { x: 457000.0, y: 339500.0 },
                coord! { x: 457500.0, y: 339500.0 },
                coord! { x: 457500.0, y: 340000.0 },
                coord! { x: 457000.0, y: 340000.0 },
                coord! { x: 457000.0, y: 339500.0 },
            ]),
            vec![],
        );
        let poly2 = Polygon::new(
            LineString::from(vec![
                coord! { x: 457500.0, y: 340000.0 },
                coord! { x: 458000.0, y: 340000.0 },
                coord! { x: 458000.0, y: 340500.0 },
                coord! { x: 457500.0, y: 340500.0 },
                coord! { x: 457500.0, y: 340000.0 },
            ]),
            vec![],
        );
        let mp = MultiPolygon::new(vec![poly1, poly2]);
        let grid = HexGrid::from_bng_multipolygon(&mp, 10);

        assert!(!grid.is_empty());
        assert_eq!(grid.zoom_level(), 10);
    }

    #[test]
    fn test_from_bng_multipolygon_deduplicates() {
        use geo_types::LineString;

        // Two overlapping polygons
        let poly1 = Polygon::new(
            LineString::from(vec![
                coord! { x: 457000.0, y: 339500.0 },
                coord! { x: 458000.0, y: 339500.0 },
                coord! { x: 458000.0, y: 340500.0 },
                coord! { x: 457000.0, y: 340500.0 },
                coord! { x: 457000.0, y: 339500.0 },
            ]),
            vec![],
        );
        let poly2 = poly1.clone();
        let mp = MultiPolygon::new(vec![poly1.clone(), poly2]);

        let mp_grid = HexGrid::from_bng_multipolygon(&mp, 10);
        let single_grid = HexGrid::from_bng_polygon(&poly1, 10);

        // Should have same number of cells due to deduplication
        assert_eq!(mp_grid.len(), single_grid.len());
    }

    #[test]
    fn test_from_wgs84_multipolygon() -> Result<(), N3gbError> {
        use geo_types::LineString;

        let poly1 = Polygon::new(
            LineString::from(vec![
                coord! { x: -2.3, y: 53.4 },
                coord! { x: -2.25, y: 53.4 },
                coord! { x: -2.25, y: 53.45 },
                coord! { x: -2.3, y: 53.45 },
                coord! { x: -2.3, y: 53.4 },
            ]),
            vec![],
        );
        let poly2 = Polygon::new(
            LineString::from(vec![
                coord! { x: -2.25, y: 53.45 },
                coord! { x: -2.2, y: 53.45 },
                coord! { x: -2.2, y: 53.5 },
                coord! { x: -2.25, y: 53.5 },
                coord! { x: -2.25, y: 53.45 },
            ]),
            vec![],
        );
        let mp = MultiPolygon::new(vec![poly1, poly2]);
        let grid = HexGrid::from_wgs84_multipolygon(&mp, 10)?;

        assert!(!grid.is_empty());
        assert_eq!(grid.zoom_level(), 10);
        Ok(())
    }

    #[test]
    fn test_builder_bng_multipolygon() {
        use geo_types::LineString;

        let poly1 = Polygon::new(
            LineString::from(vec![
                coord! { x: 457000.0, y: 339500.0 },
                coord! { x: 457500.0, y: 339500.0 },
                coord! { x: 457500.0, y: 340000.0 },
                coord! { x: 457000.0, y: 340000.0 },
                coord! { x: 457000.0, y: 339500.0 },
            ]),
            vec![],
        );
        let poly2 = Polygon::new(
            LineString::from(vec![
                coord! { x: 457500.0, y: 340000.0 },
                coord! { x: 458000.0, y: 340000.0 },
                coord! { x: 458000.0, y: 340500.0 },
                coord! { x: 457500.0, y: 340500.0 },
                coord! { x: 457500.0, y: 340000.0 },
            ]),
            vec![],
        );
        let mp = MultiPolygon::new(vec![poly1, poly2]);
        let grid = HexGrid::builder()
            .zoom_level(10)
            .bng_multipolygon(mp)
            .build();

        assert!(!grid.is_empty());
        assert_eq!(grid.zoom_level(), 10);
    }

    #[test]
    fn test_builder_wgs84_multipolygon() -> Result<(), N3gbError> {
        use geo_types::LineString;

        let poly1 = Polygon::new(
            LineString::from(vec![
                coord! { x: -2.3, y: 53.4 },
                coord! { x: -2.25, y: 53.4 },
                coord! { x: -2.25, y: 53.45 },
                coord! { x: -2.3, y: 53.45 },
                coord! { x: -2.3, y: 53.4 },
            ]),
            vec![],
        );
        let poly2 = Polygon::new(
            LineString::from(vec![
                coord! { x: -2.25, y: 53.45 },
                coord! { x: -2.2, y: 53.45 },
                coord! { x: -2.2, y: 53.5 },
                coord! { x: -2.25, y: 53.5 },
                coord! { x: -2.25, y: 53.45 },
            ]),
            vec![],
        );
        let mp = MultiPolygon::new(vec![poly1, poly2]);
        let grid = HexGrid::builder()
            .zoom_level(10)
            .wgs84_multipolygon(mp)?
            .build();

        assert!(!grid.is_empty());
        assert_eq!(grid.zoom_level(), 10);
        Ok(())
    }
}
