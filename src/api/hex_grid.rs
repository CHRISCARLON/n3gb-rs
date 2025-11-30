use crate::api::cell::HexCell;
use crate::core::constants::{GRID_EXTENTS, MAX_ZOOM_LEVEL};
use crate::core::grid::{hex_to_point, point_to_hex};
use crate::util::identifier::generate_identifier;
use geo_types::{Point, Polygon, Rect};

#[derive(Debug, Clone)]
pub struct HexGrid {
    cells: Vec<HexCell>,
    zoom_level: u8,
}

impl HexGrid {
    pub fn builder() -> HexGridBuilder {
        HexGridBuilder::new()
    }

    pub fn from_extent(min_x: f64, min_y: f64, max_x: f64, max_y: f64, zoom_level: u8) -> Self {
        let cells = generate_cells_for_extent(min_x, min_y, max_x, max_y, zoom_level);
        Self { cells, zoom_level }
    }

    pub fn from_rect(rect: &Rect<f64>, zoom_level: u8) -> Self {
        Self::from_extent(
            rect.min().x,
            rect.min().y,
            rect.max().x,
            rect.max().y,
            zoom_level,
        )
    }

    pub fn zoom_level(&self) -> u8 {
        self.zoom_level
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn cells(&self) -> &[HexCell] {
        &self.cells
    }

    pub fn iter(&self) -> impl Iterator<Item = &HexCell> {
        self.cells.iter()
    }

    pub fn get_cell_at(&self, point: &Point<f64>) -> Option<&HexCell> {
        let (row, col) = point_to_hex(point, self.zoom_level).ok()?;
        self.cells
            .iter()
            .find(|cell| cell.row == row && cell.col == col)
    }

    pub fn to_polygons(&self) -> Vec<Polygon<f64>> {
        self.cells.iter().map(|cell| cell.to_polygon()).collect()
    }

    pub fn filter<F>(&self, predicate: F) -> Vec<&HexCell>
    where
        F: Fn(&HexCell) -> bool,
    {
        self.cells.iter().filter(|cell| predicate(cell)).collect()
    }
}

#[derive(Debug, Default)]
pub struct HexGridBuilder {
    zoom_level: Option<u8>,
    min_x: Option<f64>,
    min_y: Option<f64>,
    max_x: Option<f64>,
    max_y: Option<f64>,
}

impl HexGridBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn zoom_level(mut self, zoom_level: u8) -> Self {
        self.zoom_level = Some(zoom_level);
        self
    }

    pub fn extent(mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        self.min_x = Some(min_x);
        self.min_y = Some(min_y);
        self.max_x = Some(max_x);
        self.max_y = Some(max_y);
        self
    }

    pub fn rect(mut self, rect: &Rect<f64>) -> Self {
        self.min_x = Some(rect.min().x);
        self.min_y = Some(rect.min().y);
        self.max_x = Some(rect.max().x);
        self.max_y = Some(rect.max().y);
        self
    }

    pub fn build(self) -> HexGrid {
        let zoom_level = self.zoom_level.expect("zoom_level must be set");
        let min_x = self.min_x.expect("extent must be set");
        let min_y = self.min_y.expect("extent must be set");
        let max_x = self.max_x.expect("extent must be set");
        let max_y = self.max_y.expect("extent must be set");

        HexGrid::from_extent(min_x, min_y, max_x, max_y, zoom_level)
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

    let mut hexagons = Vec::new();

    for row in min_row..=max_row {
        for col in min_col..=max_col {
            let center = match hex_to_point(row, col, zoom_level) {
                Ok(p) => p,
                Err(_) => continue,
            };

            if center.x() < GRID_EXTENTS[0] || center.y() < GRID_EXTENTS[1] {
                continue;
            }

            let id = generate_identifier(center.x(), center.y(), zoom_level);

            hexagons.push(HexCell::new(id, center, zoom_level, row, col));
        }
    }

    hexagons
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo_types::{coord, point};

    #[test]
    fn test_hex_grid_from_extent() {
        let grid = HexGrid::from_extent(457000.0, 339500.0, 458000.0, 340500.0, 10);
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
            .extent(457000.0, 339500.0, 458000.0, 340500.0)
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
        let grid = HexGrid::from_extent(457000.0, 339500.0, 458000.0, 340500.0, 10);
        let pt = point! { x: 457500.0, y: 340000.0 };

        let cell = grid.get_cell_at(&pt);
        assert!(cell.is_some());
    }

    #[test]
    fn test_filter_cells() {
        let grid = HexGrid::from_extent(457000.0, 339500.0, 458000.0, 340500.0, 10);

        let filtered = grid.filter(|cell| cell.easting() > 457500.0);
        assert!(!filtered.is_empty());
    }

    #[test]
    fn test_to_polygons() {
        let grid = HexGrid::from_extent(457000.0, 339500.0, 458000.0, 340500.0, 10);
        let polygons = grid.to_polygons();

        assert_eq!(polygons.len(), grid.len());
    }
}
