# Compare images

If the `image-compare` feature is enabled, the visual representation of objects
can be easily compared using the [`Viewport::compare_to_image`] function (only
for .png images). This is useful for testing:

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
let viewport = Viewport::from_bounding_box(&c.bounding_box(), SideLength::Long(500));
viewport.compare_to_image("docs/img/circle.png", |cr: &cairo::Context| {c.draw(cr)}, 0.99).expect("images are identical");
```

It is also possible to circumvent the usage of [`Viewport`] entirely by directly
calling the underlying free function [`compare_to_image`]. The convience wrapper
[`compare_or_create`] (also exists as method [`Viewport::compare_or_create`])
either calls [`compare_to_image`] if the specified reference image exists or
creates the file if it doesn't.