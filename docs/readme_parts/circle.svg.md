This crate adds the [`Viewport`] abstraction on top of the excellent
[cairo-rs](https://crates.io/crates/cairo-rs) crate, which itself is a Rust
wrapper around the [cairo](https://www.cairographics.org/) library.

> **Feedback welcome!**  
> Found a bug, missing docs, or have a feature request?  
> Please open an issue on GitHub.

A [`Viewport`] can be created from a [`BoundingBox`] and automatically
configures a cairo [`Context`] so it fits the underlying bounding box. This is
useful to simplify creating images of bounded objects, as shown below:

```rust
use cairo_viewport::{SideLength, Viewport};
use bounding_box::BoundingBox;

struct Circle {
    center: [f64; 2],
    radius: f64,
}

impl Circle {
    fn draw(&self, cr: &cairo::Context) -> Result<(), cairo::Error> {
        use std::f64::consts::PI;

        // Set the background to white
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.paint()?;

        cr.move_to(self.center[0] + self.radius, self.center[1]);
        cr.arc(self.center[0], self.center[1], self.radius, 0.0, PI);
        cr.arc(self.center[0], self.center[1], self.radius, PI, 0.0);
        cr.set_source_rgba(0.0, 0.0, 1.0, 1.0);
        cr.set_line_width(0.2);
        return cr.stroke();
    }

    fn bounding_box(&self) -> BoundingBox {
        return BoundingBox::new(
            self.center[0] - self.radius - 0.5,
            self.center[0] + self.radius + 0.5,
            self.center[1] - self.radius - 0.5,
            self.center[1] + self.radius + 0.5,
        );
    }
}

let c = Circle {center: [1000.0, 1000.0], radius: 2.0};

// The context is transformed so that the larger side (width or height) of the
// bounding box has 500 units when creating the image (e.g. 500 pixel for PNG).
let viewport = Viewport::from_bounding_box(&c.bounding_box(), SideLength::Long(500));

// Use the viewport to create an image
viewport.write_to_file("docs/img/circle.svg", |cr: &cairo::Context| {c.draw(cr)}).expect("image can be created");
```
File "docs/img/circle.svg":