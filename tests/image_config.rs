use bounding_box::BoundingBox;
use cairo_viewport::{SideLength, Viewport};

#[test]
#[should_panic]
fn from_infinite_bounding_box() {
    let _ = Viewport::from_bounding_box(
        &BoundingBox::new(0.0, std::f64::INFINITY, 0.0, 1.0),
        SideLength::Long(100),
    );
}

// Creating a config from an empty vector succeeds
#[test]
fn test_from_bounded_iter_empty_vec() {
    let elements: Vec<BoundingBox> = vec![];
    assert!(Viewport::from_bounded_entities(elements.into_iter(), SideLength::Long(100)).is_err());
}

#[test]
fn test_from_bounding_box() {
    {
        let max_height: u32 = 100;
        let bb = BoundingBox::new(0.0, 1.0, 0.0, 1.0);
        let config = Viewport::from_bounding_box(&bb, SideLength::Long(max_height));
        assert_eq!(config.origin, [0.0, 0.0]);
        assert_eq!(config.scale, max_height.into());
        assert_eq!(config.height, max_height);
        assert_eq!(config.width, max_height);
    }
    {
        let max_height: u32 = 100;
        let mut bb = BoundingBox::new(0.0, 1.0, 0.0, 1.0);
        bb.scale(2.0);
        let config = Viewport::from_bounding_box(&bb, SideLength::Long(max_height));
        assert_eq!(config.origin, [0.5, 0.5]);
        assert_eq!(config.scale, 0.5 * max_height as f64);
        assert_eq!(config.height, max_height);
        assert_eq!(config.width, max_height);
    }
}

#[test]
fn test_image_size_from_bounds() {
    {
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
    }
    {
        let bb = BoundingBox::new(0.0, 2.0, 0.0, 1.0); // Width of 1, height of 2

        let [width, height] = SideLength::Long(500).to_width_and_height(&bb);
        assert_eq!(width, 500);
        assert_eq!(height, 250);

        let [width, height] = SideLength::Short(500).to_width_and_height(&bb);
        assert_eq!(width, 1000);
        assert_eq!(height, 500);

        let [width, height] = SideLength::Width(500).to_width_and_height(&bb);
        assert_eq!(width, 500);
        assert_eq!(height, 250);

        let [width, height] = SideLength::Height(500).to_width_and_height(&bb);
        assert_eq!(width, 1000);
        assert_eq!(height, 500);
    }
}
