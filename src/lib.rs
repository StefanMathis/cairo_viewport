#![cfg_attr(docsrs, doc = include_str!("../README.md"))]
#![cfg_attr(not(docsrs), doc = include_str!("../README_local.md"))]
#![deny(missing_docs)]

use bounding_box::BoundingBox;
use std::{ffi::OsStr, path::Path};

// Make cairo importable from this crate
pub use cairo;

#[cfg(feature = "image-compare")]
use rand::Rng;

/**
List of image file types known to [cairo] and therefore to [`Viewport`].

[cairo] provides different [`Surface`](cairo::Surface) newtypes for the file types
in this list (e.g. [`SvgSurface`](cairo::SvgSurface) for .svg viles).
The [`Viewport::write_to_file`] struct provides a simplified interface by
reading out the file type from the provided target file path and selecting the
corresponding [cairo] implementation. If none of the listed file extensions can
be recognized in the given path, an [`Error::UnknowFileExt`] is returned.
 */
pub const CAIRO_FILE_EXTENSIONS: &[&str] = &["pdf", "png", "ps", "svg"];

/**
A viewport which can be used to scale and translate the origin of a [`cairo::Context`].

This struct simplifies the process of configuring a [`cairo::Surface`] and its
corresponding [`cairo::Context`] via the following workflow:
1) Define the bounds of the drawing with the fields [`Viewport::origin`] and
[`Viewport::origin`]. These values are used to scale and translate a
[`cairo::Context`] to make sure the image produced by [cairo] actually shows
the drawing
2) Define the dimension of the image with the fields [`Viewport::width`] and
[`Viewport::height`]. Depending on the file type used for the image, these
values have different meanings:
    - pdf: Points on the screen (1/72 inch on a screen at 100 % scale)
    - png: Pixel
    - ps: Points on the screen (1/72 inch on a screen at 100 % scale)
    - svg: CSS pixel on the screen (ca. 1/96 inch on a screen at 100 % scale)

Although this sounds very abstract, there exist several convenience constructors
which make it easy to derive the field values from e.g. a [`BoundingBox`] and
the [`SideLength`] in units:

```
use cairo_viewport::{Viewport, SideLength};
use bounding_box::BoundingBox;

/*
In this example, a Viewport is constructed from a BoundingBox which fully
covers the extents of a drawing.
 */
let bb = BoundingBox::new(6.0, 8.0, 12.0, 20.0);
let viewport = Viewport::from_bounding_box(&bb, SideLength::Long(500));

assert_eq!(viewport.origin, [-6.0, -12.0]); // Negated lower left corner of bb
assert_eq!(viewport.scale, 62.5); // (500 * 2/8) / 2
assert_eq!(viewport.width, 125);
assert_eq!(viewport.height, 500);
```
*/
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    /**
    The context is translated to these coordinates after scaling.
     */
    pub origin: [f64; 2],
    /**
    The context is scaled (both x- and y- dimension) by this value before
    translation.
     */
    pub scale: f64,
    /// Width of the surface.
    pub width: u32,
    /// Height of the surface.
    pub height: u32,
}

impl Viewport {
    /**
    Creates a [`Viewport`] from its components. This is a wrapper around the
    direct construction of the struct from its fields and does not perform any
    calculations or checks.
     */
    pub fn new(origin: [f64; 2], scale: f64, width: u32, height: u32) -> Self {
        return Viewport {
            origin,
            scale,
            width,
            height,
        };
    }

    /**
    Converts `entity` into a [`BoundingBox`] and then calls
    [`Viewport::from_bounding_box`]. See the docstring of this method for more.
     */
    pub fn from_bounded_entity<B: Into<BoundingBox>>(entity: B, side_length: SideLength) -> Self {
        return Self::from_bounding_box(&entity.into(), side_length);
    }

    /**
    Calculates the common [`BoundingBox`] of all entities and then forwards it
    to [`Viewport::from_bounding_box`]. See the docstring of this method for more.
     */
    pub fn from_bounded_entities<B: Into<BoundingBox> + ?Sized>(
        entities: impl Iterator<Item = B>,
        side_length: SideLength,
    ) -> Result<Self, &'static str> {
        return Ok(Self::from_bounding_box(
            &BoundingBox::from_bounded_entities(entities)
                .ok_or("entities iterator must yield at least one item")?,
            side_length,
        ));
    }

    /**
    Creates a [`Viewport`] from a given [`BoundingBox`] and the specified
    [`SideLength`].

    This function first calculates the `width` and `height` fields from
    [`SideLength::to_width_and_height`] using the given [`BoundingBox`].
    With these values, the `scale` factor can then be calculated. The origin
    is simply `[-bounding_box.xmin(), -bounding_box.ymin()]`.

    Constructing a [`Viewport`] in this manner ensures that any image created
    from [`Viewport`] fits the given [`BoundingBox`]. This can be used to make
    sure that a drawing (whose [`BoundingBox`] is known) exactly fills the
    image.

    # Panics
    Panics if the maximum side length is set to zero.

    # Examples

    ```
    use cairo_viewport::{Viewport, SideLength};
    use bounding_box::BoundingBox;

    let bb = BoundingBox::new(6.0, 8.0, 12.0, 20.0);

    // The width of the image is fixed to 500 units. Since the height of the
    // bounding box is four times the width, the height of the resulting image
    // is therefore 2000.
    let viewport = Viewport::from_bounding_box(&bb, SideLength::Width(500));

    assert_eq!(viewport.origin, [-6.0, -12.0]); // Negated lower left corner of bb
    assert_eq!(viewport.scale, 250.0); // 500 / 2
    assert_eq!(viewport.width, 500);
    assert_eq!(viewport.height, 2000);
    ```
    */
    pub fn from_bounding_box(bounding_box: &BoundingBox, side_length: SideLength) -> Self {
        if !bounding_box.is_finite() {
            panic!("infinite bounding box!")
        }

        let origin = [-bounding_box.xmin(), -bounding_box.ymin()];

        let [width, height] = side_length.to_width_and_height(bounding_box);

        let width_bb = bounding_box.width();
        let height_bb = bounding_box.height();
        let scale = if width_bb / height_bb > 1.0 {
            width as f64 / width_bb
        } else {
            height as f64 / height_bb
        };

        return Viewport {
            origin,
            scale,
            width,
            height,
        };
    }

    /**
    Draws an image with the given `draw_callback` and saves it into the file
    specified via `path`.

    The file type (.pdf, .png, .ps or .svg, see [`CAIRO_FILE_EXTENSIONS`]) is
    derived from the file extension specified in `path`. Hence, specifying a
    path without any of these four file extensions results in an error.

    # Examples

    ```
    use cairo_viewport::{Viewport, SideLength};
    use bounding_box::BoundingBox;

    let bb = BoundingBox::new(-1.5, 6.5, -3.5, 3.5);
    let viewport = Viewport::from_bounding_box(&bb, SideLength::Long(500));

    let draw_callback = |cr: &cairo::Context| {

        // Set the background to white
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.paint()?;

        cr.set_line_cap(cairo::LineCap::Square);

        // Draw a rectangle
        cr.move_to(-1.0, -3.0);
        cr.line_to(6.0, -3.0);
        cr.line_to(6.0, 3.0);
        cr.line_to(-1.0, 3.0);
        cr.close_path();
        cr.set_line_width(0.1);
        cr.set_source_rgba(0.0, 0.0, 1.0, 1.0); // Blue line
        cr.stroke()?;

        // Draw the origin as a black "L" shape
        cr.move_to(0.0, 0.0);
        cr.line_to(0.5, 0.0);
        cr.set_line_width(0.2);
        cr.set_source_rgba(0.0, 0.0, 0.0, 1.0); // Black line
        cr.stroke()?;

        cr.move_to(0.0, 0.0);
        cr.line_to(0.0, 0.5);
        cr.set_line_width(0.2);
        cr.set_source_rgba(0.0, 0.0, 0.0, 1.0); // Black line
        cr.stroke()?;

        return Ok(());
    };

    let path = std::path::Path::new("docs/rectangle_with_origin.svg");
    assert!(viewport.write_to_file(path, draw_callback).is_ok());
    ```
    */
    #[cfg_attr(
        docsrs,
        doc = "![](https://raw.githubusercontent.com/StefanMathis/cairo_viewport/refs/heads/main/docs/rectangle_with_origin.svg \"Rectangle with origin marker\")"
    )]
    #[cfg_attr(
        not(docsrs),
        doc = "![>> Example image missing, copy folder docs from crate root to doc root folder (where index.html is) to display the image <<](docs/rectangle_with_origin.svg)"
    )]
    pub fn write_to_file<F: FnOnce(&cairo::Context) -> Result<(), cairo::Error>, P: AsRef<Path>>(
        &self,
        path: P,
        draw_callback: F,
    ) -> Result<(), Error> {
        let path = path.as_ref();
        let extension = try_get_file_ext_for_cairo(path)?;

        // Check if the given path already points to a file. If not, try to create the file.
        let mut file = if path.exists() {
            std::fs::OpenOptions::new().write(true).open(path)?
        } else {
            std::fs::File::create(path)?
        };

        let (image_surface, cr) = match extension {
            "ps" => {
                let surface = cairo::PsSurface::new(self.width.into(), self.height.into(), path)?;
                let cr = cairo::Context::new(&surface)?;
                (None, cr)
            }
            "png" => {
                let width = self.width as i32;
                let height = self.height as i32;
                let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, width, height)?;
                let cr = cairo::Context::new(&surface)?;
                (Some(surface), cr)
            }
            "pdf" => {
                let surface = cairo::PdfSurface::new(self.width.into(), self.height.into(), path)?;
                let cr = cairo::Context::new(&surface)?;
                (None, cr)
            }
            "svg" => {
                let surface =
                    cairo::SvgSurface::new(self.width.into(), self.height.into(), Some(path))?;
                let cr = cairo::Context::new(&surface)?;
                (None, cr)
            }
            _ => unreachable!("all other possibilites filtered out in try_get_file_ext_for_cairo"),
        };

        // Adjust the context
        cr.scale(self.scale, self.scale);
        cr.translate(self.origin[0], self.origin[1]);

        // Call the callback to do the actual drawing
        draw_callback(&cr)?;

        match image_surface {
            Some(surface) => surface.write_to_png(&mut file)?,
            None => (),
        }

        return Ok(());
    }

    /**
    A wrapper around [`compare_to_image`] which uses [`Viewport::write_to_file`]
    as the `draw_callback`.

    Only available if the `image-compare` feature is enabled.

    # Examples

    ```
    use cairo_viewport::{compare_to_image, SideLength, Viewport};
    use bounding_box::BoundingBox;
    use std::path::Path;

    let bb = BoundingBox::new(-1.0, 1.0, -1.0, 1.0);
    let viewport = Viewport::from_bounding_box(&bb, SideLength::Long(500));

    fn draw_cross(cr: &cairo::Context, color: [f64; 4]) -> Result<(), cairo::Error> {

        // Set the background to white
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.paint()?;

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

    let path = "tests/img/black_cross_compare_to_image_method.png";

    // Creates the comparison image
    viewport
        .write_to_file(path, |cr| draw_cross(cr, [0.0, 0.0, 0.0, 1.0]))
        .unwrap();

    // Compare to a second black cross
    viewport.compare_to_image(path, |cr: &cairo::Context| {
        draw_cross(cr, [0.0, 0.0, 0.0, 1.0])
    })
    .expect("images should be identical");

    // Draw a blue cross
    let err = viewport.compare_to_image(path, |cr: &cairo::Context| {
        draw_cross(cr, [0.0, 0.0, 1.0, 1.0])
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
    ```
    */
    #[cfg(feature = "image-compare")]
    pub fn compare_to_image<
        F: FnOnce(&cairo::Context) -> Result<(), cairo::Error>,
        P: AsRef<Path>,
    >(
        &self,
        image: P,
        draw_callback: F,
    ) -> Result<(), Error> {
        return compare_to_image(image, |path: &Path| self.write_to_file(path, draw_callback));
    }

    /**
    A wrapper around [`compare_to_image`] which uses [`Viewport::write_to_file`]
    as the `draw_callback`.

    Only available if the `image-compare` feature is enabled.

    # Examples

    ```
    use cairo_viewport::{compare_or_create, SideLength, Viewport};
    use bounding_box::BoundingBox;
    use std::path::Path;

    let bb = BoundingBox::new(-1.0, 1.0, -1.0, 1.0);
    let viewport = Viewport::from_bounding_box(&bb, SideLength::Long(500));

    fn draw_cross(cr: &cairo::Context, color: [f64; 4]) -> Result<(), cairo::Error> {

        // Set the background to white
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.paint()?;

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

    let path = Path::new("tests/img/black_cross_compare_or_create_method.png");

    assert!(!path.exists());

    // Creates the reference image
    viewport.compare_or_create(path, |cr| draw_cross(cr, [0.0, 0.0, 0.0, 1.0])).expect("image is created");

    assert!(path.exists());

    // Reference image exists -> compare
    viewport.compare_or_create(path, |cr| draw_cross(cr, [0.0, 0.0, 0.0, 1.0])).expect("comparison succeeded");

    // Draw a blue cross
    let err = viewport.compare_or_create(path, |cr| draw_cross(cr, [0.0, 0.0, 1.0, 1.0])).unwrap_err();
    match err {
        cairo_viewport::Error::ImageCompFailed {
            reference_image: _,
            image_created_from_fn,
        } => {
            let _ = std::fs::remove_file(&image_created_from_fn).unwrap();
        }
        _ => panic!("{err}"),
    }

    // Remove the reference image
    std::fs::remove_file(&path).unwrap();
    ```
    */
    #[cfg(feature = "image-compare")]
    pub fn compare_or_create<
        F: FnOnce(&cairo::Context) -> Result<(), cairo::Error>,
        P: AsRef<Path>,
    >(
        &self,
        image: P,
        draw_callback: F,
    ) -> Result<(), Error> {
        return compare_or_create(image, |path: &Path| self.write_to_file(path, draw_callback));
    }
}

/**
Compares the image created by `draw_callback` with the one in `image`.

This function is meant to be used for testing image creation functions. If the
`draw_callback` creates the same image as the one stored in `reference_image`,
`Ok(())` is returned. In case the images are not equal,
[`Error::ImageCompFailed`] is returned. Only .png images can be compared.

Under the hood, this function uses `draw_callback` to temporarily create an
image in the same directory as `reference_image`. This image is then compared to
`reference_image` and then deleted if the comparison was successfull. Otherwise,
it is kept for further examination.

The function [`compare_or_create`] provides a convenience wrapper around this
function which creates `image` from `draw_callback` if `image` does not exist
yet.

Consider using [Viewport::compare_to_image] instead if the image is configured
using a [`Viewport`] (under the hood, [Viewport::compare_to_image] calls this
function).

Only available if the `image-compare` feature is enabled.

# Examples

```
use cairo_viewport::{compare_to_image, SideLength, Viewport};
use bounding_box::BoundingBox;
use std::path::Path;

let bb = BoundingBox::new(-1.0, 1.0, -1.0, 1.0);
let viewport = Viewport::from_bounding_box(&bb, SideLength::Long(500));

fn draw_cross(cr: &cairo::Context, color: [f64; 4]) -> Result<(), cairo::Error> {

    // Set the background to white
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.paint()?;

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

let path = "tests/img/black_cross_compare_to_image_fn.png";

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
```
*/
#[cfg(feature = "image-compare")]
pub fn compare_to_image<F: FnOnce(&Path) -> Result<(), Error>, P: AsRef<Path>>(
    reference_image: P,
    draw_callback: F,
) -> Result<(), Error> {
    fn compare_to_image_inner<F: FnOnce(&Path) -> Result<(), Error>>(
        p: &std::path::Path,
        tmp_image: &std::path::Path,
        draw_callback: F,
    ) -> Result<(), Error> {
        // Populate the file
        draw_callback(&tmp_image)?;

        // Open the images
        let image_one = image::open(&p)?.into_luma8();
        let image_two = image::open(&tmp_image)?.into_luma8();

        // Compare the images
        let result = image_compare::gray_similarity_structure(
            &image_compare::Algorithm::MSSIMSimple,
            &image_one,
            &image_two,
        )?;

        // result.score = 1 means the images are identical
        if result.score > 0.95 {
            return Ok(());
        } else {
            return Err(Error::ImageCompFailed {
                reference_image: p.to_path_buf(),
                image_created_from_fn: tmp_image.to_path_buf(),
            });
        }
    }

    let p = reference_image.as_ref();

    // Try to get the file extension
    let ext = try_get_file_ext_for_cairo(p)?;

    if ext != "png" {
        return Err(Error::UnknowFileExt(
            "when comparing images, only .png images are allowed.".into(),
        ));
    }

    // Temporary path: Remove the file extension
    let tmp_image = p.with_extension("");
    let tmp_image = tmp_image.as_os_str();
    let mut tmp_image = tmp_image
        .to_str()
        .ok_or(Error::InvalidFilename(tmp_image.to_owned()))?
        .to_owned();
    tmp_image.push_str("_TEST_");
    tmp_image.push_str(&create_random_filename(30));
    let mut tmp_image = Path::new(&tmp_image).to_owned();
    tmp_image.set_extension(ext);

    // Create the temporary file.
    let _ = std::fs::File::create(&tmp_image)?;

    let _ = compare_to_image_inner(p, &tmp_image, draw_callback)?;
    std::fs::remove_file(&tmp_image)?;
    return Ok(());
}

/**
Wrapper around [`compare_to_image`] which checks if the specfied
`reference_image` exists. If it does, the arguments are forwared to
[`compare_to_image`]. If not, the image is simply created, using
`draw_callback`.

Only available if the `image-compare` feature is enabled.

# Examples

```
use cairo_viewport::{compare_or_create, SideLength, Viewport};
use bounding_box::BoundingBox;
use std::path::Path;

let bb = BoundingBox::new(-1.0, 1.0, -1.0, 1.0);
let viewport = Viewport::from_bounding_box(&bb, SideLength::Long(500));

fn draw_cross(cr: &cairo::Context, color: [f64; 4]) -> Result<(), cairo::Error> {

    // Set the background to white
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.paint()?;

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

let path = Path::new("tests/img/black_cross_compare_or_create_fn.png");

assert!(!path.exists());

// Creates the reference image
compare_or_create(path, |path: &Path| {
    viewport.write_to_file(path, |cr| draw_cross(cr, [0.0, 0.0, 0.0, 1.0]))
}).expect("image is created");

assert!(path.exists());

// Reference image exists -> compare
compare_or_create(path, |path: &Path| {
    viewport.write_to_file(path, |cr| draw_cross(cr, [0.0, 0.0, 0.0, 1.0]))
}).expect("comparison succeeded");

// Draw a blue cross
let err = compare_or_create(path, |path: &Path| {
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

// Remove the reference image
std::fs::remove_file(&path).unwrap();
```
*/
#[cfg(feature = "image-compare")]
pub fn compare_or_create<F: FnOnce(&Path) -> Result<(), Error>, P: AsRef<Path>>(
    reference_image: P,
    draw_callback: F,
) -> Result<(), Error> {
    let p = reference_image.as_ref();

    // Create the file anew, if necessary
    if p.exists() {
        return compare_to_image(&p, draw_callback);
    } else {
        // Check if the given path already points to a file. If not, try to create the file.
        let _ = if p.exists() {
            // https://users.rust-lang.org/t/os-error-5-when-writing-to-file-on-windows-10/49307/2
            std::fs::OpenOptions::new().write(true).open(p)?
        } else {
            std::fs::File::create(p)?
        };
        draw_callback(p)?;

        return Ok(());
    }
}

#[cfg(feature = "image-compare")]
fn create_random_filename(name_length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789";
    let mut rng = rand::thread_rng();

    let filename: String = (0..name_length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    return filename;
}

fn try_get_file_ext_for_cairo(path: &Path) -> Result<&str, Error> {
    match path.extension().and_then(OsStr::to_str) {
        Some(ext) => {
            // Check if the provided file extension matches one of the available file extension types
            if !CAIRO_FILE_EXTENSIONS.contains(&ext) {
                let mut msg = format!(
                    "The given file extension \"{}\" is not recognized. Available file extensions are: ",
                    &ext
                );
                for (i, ext) in CAIRO_FILE_EXTENSIONS.iter().enumerate() {
                    msg.push_str(ext);
                    if (i + 1) < CAIRO_FILE_EXTENSIONS.len() {
                        msg.push_str(", ");
                    }
                }
                return Err(Error::UnknowFileExt(msg));
            }
            return Ok(ext);
        }
        None => {
            let mut msg: String =
                "No file extension has been recognized. Add one of the following file extensions: "
                    .into();
            for (i, ext) in CAIRO_FILE_EXTENSIONS.iter().enumerate() {
                msg.push_str(ext);
                if (i + 1) < CAIRO_FILE_EXTENSIONS.len() {
                    msg.push_str(", ");
                }
            }
            return Err(Error::UnknowFileExt(msg));
        }
    };
}

/**
Calculation of the image size from side length and [`BoundingBox`].

This enum specifies the length of one of the image sides in units. The length of
the second side can then be derived from a [`BoundingBox`], see
[`SideLength::to_width_and_height`] for an example.

The exact meaning of "units" depends on the selected output format:
- pdf: Points on the screen (1/72 inch on a screen at 100 % scale)
- png: Pixel
- ps: Points on the screen (1/72 inch on a screen at 100 % scale)
- svg: CSS pixel on the screen (ca. 1/96 inch on a screen at 100 % scale)
 */
#[derive(Debug, Copy, Clone)]
pub enum SideLength {
    /// Fixed length for the long side of the bounding box
    Long(u32),
    /// Fixed length for the short side of the bounding box
    Short(u32),
    /// Fixed length for the width of the bounding box
    Width(u32),
    /// Fixed length for the height of the bounding box
    Height(u32),
}

impl From<SideLength> for u32 {
    fn from(value: SideLength) -> Self {
        match value {
            SideLength::Long(v) => v,
            SideLength::Short(v) => v,
            SideLength::Width(v) => v,
            SideLength::Height(v) => v,
        }
    }
}

impl SideLength {
    /**
    Calculates image width and height from `self` and a [`BoundingBox`].
    ```
    use cairo_viewport::SideLength;
    use bounding_box::BoundingBox;

    let bb = BoundingBox::new(0.0, 1.0, 0.0, 2.0); // Width of 1, height of 2

    let [width, height] = SideLength::Long(500).to_width_and_height(&bb);
    assert_eq!(width, 250);
    assert_eq!(height, 500);

    let [width, height] = SideLength::Short(500).to_width_and_height(&bb);
    assert_eq!(width, 500);
    assert_eq!(height, 1000);

    let [width, height] = SideLength::Width(500).to_width_and_height(&bb);
    assert_eq!(width, 500);
    assert_eq!(height, 1000);

    let [width, height] = SideLength::Height(500).to_width_and_height(&bb);
    assert_eq!(width, 250);
    assert_eq!(height, 500);
    ```
     */
    pub fn to_width_and_height(&self, bounding_box: &BoundingBox) -> [u32; 2] {
        let ratio = bounding_box.width() / bounding_box.height();
        let mut width: u32;
        let mut height: u32;
        match self {
            SideLength::Long(l) => {
                if ratio > 1.0 {
                    width = *l;
                    height = (*l as f64 / ratio).ceil() as u32;
                } else {
                    width = (*l as f64 * ratio).ceil() as u32;
                    height = *l;
                }
            }
            SideLength::Short(l) => {
                if ratio > 1.0 {
                    width = (*l as f64 * ratio).ceil() as u32;
                    height = *l;
                } else {
                    width = *l;
                    height = (*l as f64 / ratio).ceil() as u32;
                }
            }
            SideLength::Width(w) => {
                width = *w;
                height = (*w as f64 / ratio).ceil() as u32;
            }
            SideLength::Height(h) => {
                width = (*h as f64 * ratio).ceil() as u32;
                height = *h;
            }
        }

        // If the length or width of the bounding box is zero, the width or height of the image configuration is zero as well.
        // Therefore, zero values are set to 1.
        if width == 0 {
            width = 1;
        }
        if height == 0 {
            height = 1;
        }

        return [width, height];
    }
}

/**
Errors which may occur when using [`Viewport`].
 */
#[derive(Debug)]
pub enum Error {
    /// Error from a call to a cairo-rs method.
    CairoError(cairo::Error),
    /// Specified file extension is not valid.
    UnknowFileExt(String),
    /// An IO error returned from the file system.
    IoError(std::io::Error),
    /// Specified filename is invalid
    InvalidFilename(std::ffi::OsString),
    /// Error returned by [`compare_to_image`] and related functions.
    /// It indicates that the reference image found in the given path does not
    /// match that created by the drawing function.
    #[cfg(feature = "image-compare")]
    ImageCompFailed {
        /// Reference image.
        reference_image: std::path::PathBuf,
        /// The image created by the drawing function
        image_created_from_fn: std::path::PathBuf,
    },
    /// An error occurred when trying to open that image
    #[cfg(feature = "image-compare")]
    ImageError(image::ImageError),
    /// An error occurred when using the image-compare crate.
    #[cfg(feature = "image-compare")]
    CompareError(image_compare::CompareError),
}

impl From<cairo::Error> for Error {
    fn from(value: cairo::Error) -> Self {
        Error::CairoError(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IoError(value)
    }
}

impl From<cairo::IoError> for Error {
    fn from(value: cairo::IoError) -> Self {
        match value {
            cairo::IoError::Cairo(error) => error.into(),
            cairo::IoError::Io(error) => error.into(),
        }
    }
}

#[cfg(feature = "image-compare")]
impl From<image_compare::CompareError> for Error {
    fn from(value: image_compare::CompareError) -> Self {
        Error::CompareError(value)
    }
}

#[cfg(feature = "image-compare")]
impl From<image::ImageError> for Error {
    fn from(value: image::ImageError) -> Self {
        Error::ImageError(value)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::CairoError(error) => error.fmt(f),
            Error::UnknowFileExt(str) => write!(f, "{str}"),
            Error::IoError(error) => error.fmt(f),
            Error::InvalidFilename(os_string) => {
                write!(f, "invalid filename {}", os_string.to_string_lossy())
            }
            #[cfg(feature = "image-compare")]
            Error::ImageCompFailed {
                reference_image,
                image_created_from_fn,
            } => {
                write!(
                    f,
                    "image {} created by the function does not match the reference image {}",
                    image_created_from_fn.display(),
                    reference_image.display(),
                )
            }
            #[cfg(feature = "image-compare")]
            Error::CompareError(error) => error.fmt(f),
            #[cfg(feature = "image-compare")]
            Error::ImageError(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {}
