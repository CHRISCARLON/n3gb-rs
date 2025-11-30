/// Identifier version for encoding/decoding
pub const IDENTIFIER_VERSION: u8 = 1;

/// Scale factor to preserve three decimal places
pub(crate) const SCALE_FACTOR: u64 = 1000;

/// Grid extents [min_x, min_y, max_x, max_y]
pub const GRID_EXTENTS: [f64; 4] = [0.0, 0.0, 750000.0, 1350000.0];

/// Cell radius for each zoom level (0-15)
pub const CELL_RADIUS: [f64; 16] = [
    1281249.9438829257,
    483045.8762201923,
    182509.65769514776,
    68979.50076169973,
    26069.67405498836,
    9849.595592375015,
    3719.867784388759,
    1399.497052515653,
    529.4301968468868,
    199.76319313961054,
    75.05553499465135,
    28.290163190291665,
    10.392304845413264,
    4.041451884327381,
    1.7320508075688774,
    0.5773502691896258,
];

/// Cell widths for each zoom level (0-15)
pub const CELL_WIDTHS: [f64; 16] = [
    2219190.0, 836660.0, 316116.0, 119476.0, 45154.0, 17060.0, 6443.0, 2424.0, 917.0, 346.0,
    130.0, 49.0, 18.0, 7.0, 3.0, 1.0,
];

/// Maximum zoom level
pub const MAX_ZOOM_LEVEL: u8 = 15;
