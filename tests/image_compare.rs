use bounding_box::BoundingBox;
use cairo_viewport::{SideLength, Viewport, compare_to_image};
use std::path::Path;

#[test]
fn test_compare_image() {
    let bb = BoundingBox::new(-1.0, 1.0, -1.0, 1.0);
    let viewport = Viewport::from_bounding_box(&bb, SideLength::Long(500));

    fn draw_cross(cr: &cairo::Context, color: [f64; 4]) -> Result<(), cairo::Error> {
        cr.move_to(-1.0, 0.0);
        cr.line_to(1.0, 0.0);
        cr.set_line_width(0.2);
        cr.set_source_rgba(color[0], color[1], color[2], color[3]);
        cr.stroke()?;

        cr.move_to(0.0, -1.0);
        cr.line_to(0.0, 1.0);
        cr.set_line_width(0.2);
        cr.set_source_rgba(color[0], color[1], color[2], color[3]);
        cr.stroke()?;

        return Ok(());
    }

    let path = "tests/img/black_cross.png";

    // Creates the comparison image
    viewport
        .write_to_file(path, |cr| draw_cross(cr, [0.0, 0.0, 0.0, 1.0]))
        .unwrap();

    // Compare to a second black cross
    compare_to_image(path, |path: &Path| {
        viewport.write_to_file(path, |cr| draw_cross(cr, [0.0, 0.0, 0.0, 1.0]))
    })
    .expect("images should be identical");

    // Draw a blue cross
    let err = compare_to_image(path, |path: &Path| {
        viewport.write_to_file(path, |cr| draw_cross(cr, [0.0, 0.0, 1.0, 1.0]))
    })
    .unwrap_err();
    match err {
        cairo_viewport::Error::ImageCompFailed {
            reference_image: _,
            image_created_from_fn,
        } => {
            let _ = std::fs::remove_file(&image_created_from_fn).unwrap();
        }
        _ => panic!("{err}"),
    }
}
